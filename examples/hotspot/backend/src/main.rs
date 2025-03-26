use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use config::{Config, ConfigError, Environment};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use validator::{Validate, ValidateArgs, ValidationError};

use unifi_client::{models, UniFiClient};

// Context struct for validation limits
#[derive(Clone)]
struct ValidationLimits {
    max_duration_minutes: u32,
    max_data_quota_megabytes: u64,
}

fn validate_duration(
    duration_minutes: u32,
    context: &ValidationLimits,
) -> Result<(), ValidationError> {
    if duration_minutes < 1 {
        return Err(ValidationError::new("Duration must be positive"));
    }
    if duration_minutes > context.max_duration_minutes {
        let mut err = ValidationError::new("duration_too_large");
        err.message = Some(
            format!(
                "Duration must be between 1 and {} minutes",
                context.max_duration_minutes
            )
            .into(),
        );
        return Err(err);
    }
    Ok(())
}

fn validate_data_quota(
    data_quota_megabytes: u64,
    context: &ValidationLimits,
) -> Result<(), ValidationError> {
    if data_quota_megabytes < 1 {
        return Err(ValidationError::new("Data quota must be positive"));
    }
    if data_quota_megabytes > context.max_data_quota_megabytes {
        let mut err = ValidationError::new("quota_too_large");
        err.message = Some(
            format!(
                "Data quota must be between 1 and {} MB",
                context.max_data_quota_megabytes
            )
            .into(),
        );
        return Err(err);
    }
    Ok(())
}

#[derive(Debug, Validate, Deserialize)]
#[serde(deny_unknown_fields)]
#[validate(context = ValidationLimits)]
struct GuestAuthRequest {
    #[validate(
        regex(
            path = *MAC_ADDRESS_REGEX,
            message = "MAC address must be in format 00:11:22:33:44:55 with colons and exactly two hex digits per segment"
        )
    )]
    client_mac_address: String,

    #[validate(custom(function = validate_duration, use_context))]
    duration_minutes: Option<u32>,

    #[validate(custom(function = validate_data_quota, use_context))]
    data_quota_megabytes: Option<u64>,

    #[validate(
        regex(
            path = *MAC_ADDRESS_REGEX,
            message = "MAC address must be in format 00:11:22:33:44:55 with colons and exactly two hex digits per segment"
        )
    )]
    access_point_mac_address: String,

    #[validate(range(
        min = 1735689600_i64,
        max = 4102444800_i64,
        message = "Captive portal timestamp must be between 1735689600 (2025-01-01 00:00:00 UTC) and 4102444800 (2050-01-01 00:00:00 UTC)"
    ))]
    captive_portal_timestamp: i64, // Unix timestamp in seconds

    #[validate(length(min = 8, max = 2048))]
    requested_url: String, // URL client was attempting to access

    #[validate(length(min = 1, max = 32))]
    wifi_network: String,
}

static MAC_ADDRESS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9A-Fa-f]{2}:){5}([0-9A-Fa-f]{2})$").unwrap());

#[derive(Serialize)]
struct GuestAuthResponse {
    expires_at: i64,
    guest_id: String,
}

// Configuration struct
#[derive(Clone, Debug, Deserialize)]
struct AppConfig {
    unifi_controller_url: String,
    unifi_username: String,
    unifi_site: String,
    verify_ssl: bool,
    port: u16,
    max_duration_minutes: u32,
    max_data_quota_megabytes: u64,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        // Load .env file first
        dotenv().ok();

        let config = Config::builder()
            // Add in settings from environment variables
            .add_source(Environment::default())
            .build()?;

        // Convert the config values into our Config struct
        config.try_deserialize()
    }
}

// Shared application state
#[derive(Clone)]
struct AppState {
    config: AppConfig,
    unifi_client: Arc<UniFiClient>,
}

async fn authorize_guest(
    State(state): State<AppState>,
    Json(payload): Json<GuestAuthRequest>,
) -> Result<(StatusCode, Json<GuestAuthResponse>), (StatusCode, String)> {
    
    // Create validation context with limits from config
    let validation_limits = ValidationLimits {
        max_duration_minutes: state.config.max_duration_minutes,
        max_data_quota_megabytes: state.config.max_data_quota_megabytes,
    };

    // Validate the payload
    if let Err(errors) = payload.validate_with_args(&validation_limits) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid request parameters: {}", errors),
        ));
    }

    // Authorize the guest.
    let mut auth_builder = state.unifi_client
        .guests()
        .authorize(payload.client_mac_address)
        .access_point_mac_address(payload.access_point_mac_address)
        .captive_portal_timestamp(payload.captive_portal_timestamp)
        .requested_url(payload.requested_url)
        .wifi_network(payload.wifi_network);

    // Conditionally set duration if provided.
    if let Some(duration) = payload.duration_minutes {
        auth_builder = auth_builder.duration_minutes(duration);
    }
    
    // Conditionally set data quota if provided.
    if let Some(data_quota) = payload.data_quota_megabytes {
        auth_builder = auth_builder.data_quota_megabytes(data_quota);
    }

    // Execute the request.
    let guest_entry = auth_builder
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to authorize guest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to authorize guest: {}", e),
            )
        })?;

    // Return guest authorization response.
    match guest_entry {
        models::guests::GuestEntry::New { id, end, mac, .. } => {
            let expiration_time = DateTime::<Utc>::from_timestamp(end, 0).unwrap_or_default();

            let quota_info = if let Some(quota) = payload.data_quota_megabytes {
                format!(" with data quota of {} MB", quota)
            } else {
                String::new()
            };

            tracing::info!(
                "Successfully authorized guest {} until {} UTC{}",
                mac,
                expiration_time.format("%Y-%m-%d %H:%M:%S"),
                quota_info
            );

            Ok((
                StatusCode::CREATED,
                Json(GuestAuthResponse {
                    expires_at: end,
                    guest_id: id,
                }),
            ))
        }
        unexpected => {
            tracing::error!("Unexpected guest entry type: {:?}", unexpected);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected guest entry type received".to_string(),
            ))
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load application configuration from environment variables.
    let config = AppConfig::new().expect("Failed to load configuration");
    tracing::info!(
        "Starting UniFi Cafe backend with configuration: {:?}",
        config
    );

    // Create and initialize the UniFi client.
    let unifi_client = UniFiClient::builder()
        .controller_url(&config.unifi_controller_url)
        .username(&config.unifi_username)
        .password_from_env("UNIFI_PASSWORD")
        .site(&config.unifi_site)
        .verify_ssl(config.verify_ssl)
        .build()
        .await
        .expect("Failed to build UniFiClient");
    unifi_client::initialize(unifi_client);
    tracing::info!("UniFi client initialized successfully!");

    // Create shared state with the authenticated UniFi client and session store.
    let state = AppState {
        config: config.clone(),
        unifi_client: unifi_client::instance()
    };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the Axum application with the guest authorization endpoint.
    let app = Router::new()
        .route("/guest/authorize", post(authorize_guest))
        .layer(cors)
        .with_state(state);

    // Bind the server to localhost:3000.
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
