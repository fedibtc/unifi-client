use std::fmt;
use std::net::SocketAddr;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use config::{Config, ConfigError, Environment};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use unifi_client::{ClientConfig, GuestConfig, GuestEntry, UnifiClient};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct GuestAuthRequest {
    #[validate(length(min = 12, max = 17, message = "MAC address must be in a valid format"))]
    mac_address: String,
    
    #[validate(range(min = 1, max = 43200, message = "Duration must be between 1 and 43200 minutes (30 days)"))]
    duration_minutes: Option<u32>,
    
    #[validate(range(min = 1, message = "Data quota must be positive"))]
    data_quota_megabytes: Option<u64>,
}

#[derive(Serialize)]
struct GuestAuthResponse {
    data_quota: Option<u64>,
    expires_at: i64,
    guest_id: String,
    mac: String,
}

// Configuration struct
#[derive(Deserialize, Clone)]
struct AppConfig {
    unifi_controller_url: String,
    unifi_username: String,
    unifi_password: String,
    unifi_site: String,
    verify_ssl: bool,
    port: u16,
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

// Custom Debug implementation that masks the password
impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("unifi_controller_url", &self.unifi_controller_url)
            .field("unifi_username", &self.unifi_username)
            .field("unifi_password", &"******") // Mask password
            .field("unifi_site", &self.unifi_site)
            .field("verify_ssl", &self.verify_ssl)
            .field("port", &self.port)
            .finish()
    }
}

// Shared application state
#[derive(Clone)]
struct AppState {
    unifi_client: std::sync::Arc<Mutex<UnifiClient>>,
}

async fn authorize_guest(
    State(state): State<AppState>,
    Json(payload): Json<GuestAuthRequest>,
) -> Result<Json<GuestAuthResponse>, (StatusCode, String)> {
    // Validate the payload
    if let Err(errors) = payload.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid request parameters: {}", errors),
        ));
    }

    // Build the guest config.
    let mut config_builder = GuestConfig::builder().mac(&payload.mac_address);
    if let Some(duration) = payload.duration_minutes {
        config_builder = config_builder.duration(duration);
    }
    if let Some(quota) = payload.data_quota_megabytes {
        config_builder = config_builder.data_quota(quota);
    }
    let guest_config = config_builder
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build guest config: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build guest config: {}", e),
            )
        })
        .unwrap();

    // Authorize the guest.
    let client = state.unifi_client.lock().await;
    let guest_entry = client
        .guests()
        .authorize(guest_config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to authorize guest: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to authorize guest: {}", e),
            )
        })
        .unwrap();

    // Return guest authorization response.
    match guest_entry {
        GuestEntry::New { id, end, mac, .. } => {
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

            Ok(Json(GuestAuthResponse {
                data_quota: payload.data_quota_megabytes,
                expires_at: end,
                guest_id: id,
                mac,
            }))
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

    // Build the UniFi client configuration.
    let unifi_client_config = ClientConfig::builder()
        .controller_url(&config.unifi_controller_url)
        .username(&config.unifi_username)
        .site(&config.unifi_site)
        .verify_ssl(config.verify_ssl)
        .build()
        .expect("Failed to build UniFi client configuration");

    tracing::info!(
        "Initializing UniFi client with configuration: {:?}",
        unifi_client_config
    );
    let mut unifi_client = UnifiClient::new(unifi_client_config);

    // Login to the UniFi controller.
    unifi_client
        .login(Some(config.unifi_password.clone()))
        .await
        .expect("Failed to authenticate with UniFi controller");
    tracing::info!("Authentication successful!");

    // Create shared state with the authenticated UniFi client and session store.
    let state = AppState {
        unifi_client: std::sync::Arc::new(Mutex::new(unifi_client)),
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
