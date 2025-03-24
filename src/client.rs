use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
use reqwest::{Client as ReqwestClient, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Mutex;
use url::Url;

use crate::api::guest;
use crate::models::{ApiResponse, EmptyResponse};
use crate::{models, UniFiError, UniFiResult};

static UNIFI_CLIENT: Lazy<ArcSwap<UniFiClient>> = Lazy::new(|| {
    // Create a default client using the builder's default values.
    ArcSwap::new(Arc::new(UniFiClient::default()))
});

/// Initializes the static UniFiClient instance.  This should be called once
/// at the beginning of your application.
pub fn initialize(client: UniFiClient) {
    UNIFI_CLIENT.store(Arc::new(client));
}

/// Returns a reference to the static UniFiClient instance.
///
/// This function provides a thread-safe way to access the UniFi client
/// instance. It returns a reference to the current UniFi client, which can be
/// used to make API requests. If it hasn't been previously initialized it
/// returns a default instance with no authentication set.
pub fn instance() -> Arc<UniFiClient> {
    UNIFI_CLIENT.load_full()
}

/// Builder for UniFi client.
///
/// This builder provides a fluent API for creating UniFi clients
/// with validation at build time.
#[derive(Default)]
pub struct UniFiClientBuilder {
    controller_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    site: Option<String>,
    verify_ssl: bool,
    timeout: Option<Duration>,
    user_agent: Option<String>,
    http_client: Option<ReqwestClient>,
}

impl UniFiClientBuilder {
    /// Sets the controller URL.
    pub fn controller_url(mut self, url: impl Into<String>) -> Self {
        self.controller_url = Some(url.into());
        self
    }

    /// Sets the username for authentication.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password for authentication.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the password from an environment variable.
    pub fn password_from_env(mut self, var_name: &str) -> Self {
        let password = std::env::var(var_name)
            .map_err(|e| format!("Failed to read environment variable '{}': {}", var_name, e))
            .expect("Failed to set password from environment");
        self.password = Some(password);
        self
    }

    /// Sets the site to use.
    pub fn site(mut self, site: impl Into<String>) -> Self {
        self.site = Some(site.into());
        self
    }

    /// Sets whether to verify SSL certificates.
    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.verify_ssl = verify;
        self
    }

    /// Sets the HTTP request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets a custom user agent string.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Sets a custom reqwest client (e.g., for testing or custom middleware).
    pub fn http_client(mut self, http_client: ReqwestClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    pub async fn build(self) -> UniFiResult<UniFiClient> {
        let site = self.site.unwrap_or_else(|| "default".to_string());

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));
        
        let password = self.password.ok_or_else(|| {
            UniFiError::ConfigurationError("Password is required".into())
        })?;
        
        let username = self.username.ok_or_else(|| {
            UniFiError::ConfigurationError("Username is required".into())
        })?;

        let controller_url = self.controller_url.ok_or_else(||
            UniFiError::ConfigurationError("Controller URL is required".into()))
            .and_then(|url_str| {
                Url::parse(&url_str).map_err(|e| {
                    UniFiError::ConfigurationError(format!("Invalid controller URL: {e}"))
                })
            })?;

        let user_agent = self.user_agent
            .as_deref()
            .unwrap_or(concat!("unifi-client/", env!("CARGO_PKG_VERSION")));

        let http_client = if let Some(custom_client) = self.http_client {
            custom_client
        } else {
            ReqwestClient::builder()
                .timeout(timeout)
                .danger_accept_invalid_certs(!self.verify_ssl)
                .cookie_store(true)
                .user_agent(user_agent)
                .build()
                .map_err(|e| {
                    UniFiError::ConfigurationError(format!("Failed to create HTTP client: {e}"))
                })?
        };

        let client = UniFiClient {
            controller_url,
            username,
            password: Some(password),
            site,
            verify_ssl: self.verify_ssl,
            timeout,
            user_agent: self.user_agent,
            http_client,
            auth_state: Arc::new(Mutex::new(None)),
        };
        client.login().await?;
        Ok(client)
    }
}

/// Authentication state for the client.
#[derive(Clone, Debug)]
struct AuthState {
    cookies: String,
    csrf_token: Option<String>,
}

/// The main UniFi client for interacting with the UniFi Controller API.
///
/// This client manages authentication, request handling, and provides access
/// to the various API endpoints through dedicated API objects.
pub struct UniFiClient {
    controller_url: Url,
    username: String,
    password: Option<String>,
    site: String,
    verify_ssl: bool,
    timeout: Duration,
    user_agent: Option<String>,
    http_client: ReqwestClient,
    auth_state: Arc<Mutex<Option<AuthState>>>,
}

impl Default for UniFiClient {
    fn default() -> Self {
        UniFiClient {
            controller_url: Url::parse("https://localhost:8443").expect("Failed to parse default URL"),
            username: "admin".to_string(),
            password: Some("admin".to_string()),
            site: "default".to_string(),
            verify_ssl: false,
            timeout: Duration::from_secs(30),
            user_agent: Some(concat!("unifi-client/", env!("CARGO_PKG_VERSION")).to_string()),
            http_client: reqwest::Client::new(),
            auth_state: Arc::new(Mutex::new(None)),
        }
    }
}

impl fmt::Debug for UniFiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auth_state_locked = self.auth_state.blocking_lock();

        f.debug_struct("UniFiClient")
            .field("controller_url", &self.controller_url)
            .field("username", &self.username)
            .field("site", &self.site)
            .field("verify_ssl", &self.verify_ssl)
            .field("timeout", &self.timeout)
            .field("user_agent", &self.user_agent)
            .field("auth_state", &auth_state_locked.is_some())
            .finish()
    }
}

impl UniFiClient {
    pub fn builder() -> UniFiClientBuilder {
        UniFiClientBuilder::default()
    }

    async fn login(&self) -> UniFiResult<()> {
        let password = match &self.password {
            Some(pwd) => pwd.clone(),
            None => {
                return Err(UniFiError::AuthenticationError(
                    "No password provided for authentication".into(),
                ))
            }
        };

        let login_url =
            self.controller_url.join("/api/login").map_err(|e| UniFiError::UrlParseError(e))?;

        let login_data = models::auth::LoginRequest {
            username: self.username.clone(),
            password,
        };

        let response = self.http_client.post(login_url).json(&login_data).send().await?;

        if !response.status().is_success() {
            return Err(UniFiError::AuthenticationError(format!(
                "Authentication failed with status code: {}",
                response.status()
            )));
        }

        let cookie_header = response
            .headers()
            .get("set-cookie")
            .ok_or_else(|| {
                UniFiError::AuthenticationError("No cookies received from server".into())
            })?
            .to_str()
            .map_err(|e| UniFiError::AuthenticationError(format!("Invalid cookie header: {}", e)))?
            .to_string();

        let csrf_token = response
            .headers()
            .get("x-csrf-token")
            .map(|v| v.to_str().unwrap_or_default().to_string());

        let login_response: ApiResponse<Vec<EmptyResponse>> = response.json().await?;

        if login_response.meta.rc != "ok" {
            return Err(UniFiError::AuthenticationError(
                login_response.meta.msg.unwrap_or_else(|| "Unknown error".into()),
            ));
        }

        let mut auth_state = self.auth_state.lock().await;
        *auth_state = Some(AuthState {
            cookies: cookie_header,
            csrf_token,
        });

        Ok(())
    }

    /// Ensure the client is authenticated.
    async fn ensure_authenticated(&self) -> UniFiResult<()> {
        // Check if essential fields are configured *before* trying to use them.
        if self.username.is_empty() {
            return Err(UniFiError::ConfigurationError("Username is required".into()));
        }
        if self.controller_url.as_str().is_empty() {
            return Err(UniFiError::ConfigurationError("Controller URL is required".into()));
        }
        if self.auth_state.lock().await.is_none() {
            return self.login().await;
        }

        let url =
            self.controller_url.join("/api/self").map_err(|e| UniFiError::UrlParseError(e))?;

        match self.http_client.get(url).headers(self.get_auth_headers().await?).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(());
                } else if response.status() == StatusCode::UNAUTHORIZED {
                    return self.login().await;
                }
                Ok(())
            }
            Err(_) => self.login().await,
        }
    }

    // Helper to get authentication headers
    async fn get_auth_headers(&self) -> UniFiResult<HeaderMap> {
        let auth_state = self.auth_state.lock().await;
        let auth_state = auth_state.as_ref().ok_or(UniFiError::NotAuthenticated)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&auth_state.cookies)
                .map_err(|e| UniFiError::ApiError(format!("Invalid cookie header: {}", e)))?,
        );

        if let Some(token) = &auth_state.csrf_token {
            headers.insert(
                "x-csrf-token",
                HeaderValue::from_str(token)
                    .map_err(|e| UniFiError::ApiError(format!("Invalid CSRF token: {}", e)))?,
            );
        }

        Ok(headers)
    }

    /// Makes a raw request to the UniFi API.
    ///
    /// # Warning
    ///
    /// This is an advanced API that bypasses the type-safe wrappers.
    /// Use the typed API methods (like `guests()`, `sites()`) when possible.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to use (e.g., "GET", "POST").
    /// * `endpoint` - The API endpoint path (e.g.,
    ///   "/api/s/default/stat/sysinfo").
    /// * `body` - Optional request body (must implement `Serialize`).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not configured (missing URL, username, etc.).
    /// - Authentication fails (invalid credentials, expired session).
    /// - The request fails due to network issues.
    /// - The API returns an error response.
    /// - Deserialization of the response fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::{UniFiClient, UniFiError};
    /// # use serde_json::Value;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), UniFiError> {
    /// // You MUST initialize the client *before* using instance().
    /// //  For real use, you'd get the password from a secure location,
    /// //  not hardcode it.
    /// let client = UniFiClient::builder()
    ///     .controller_url("https://your-controller-url:8443")
    ///     .username("your_username")
    ///     .password("your_password")
    ///     .build()
    ///     .await?;
    /// unifi_client::initialize(client);
    ///
    /// // Get system status with a raw request.
    /// // The result is a serde_json::Value.
    /// let status: Value = unifi_client::instance()
    ///     .raw_request("GET", "/api/s/default/stat/sysinfo", None::<()>)
    ///     .await?;
    ///
    /// println!("System info: {:?}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn raw_request<T>(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<Value>
    where
        T: Serialize,
    {
        self.ensure_authenticated().await?;

        let url = self.controller_url.join(endpoint).map_err(|e| UniFiError::UrlParseError(e))?;

        let mut request = self
            .http_client
            .request(Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET), url);

        request = request.headers(self.get_auth_headers().await?);

        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;
        let api_response: ApiResponse<Value> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UniFiError::ApiError(
                api_response.meta.msg.unwrap_or_else(|| "Unknown API error".into()),
            ));
        }

        Ok(api_response.data.unwrap_or(Value::Null))
    }

    /// Make a request to the UniFi API.
    pub(crate) async fn request<T, R>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.ensure_authenticated().await?;

        let url = self.controller_url.join(endpoint).map_err(|e| UniFiError::UrlParseError(e))?;

        let mut request = self.http_client.request(method, url);

        request = request.headers(self.get_auth_headers().await?);

        // Add JSON body if provided
        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(UniFiError::NotAuthenticated);
        }

        if !response.status().is_success() {
            return Err(UniFiError::ApiError(format!(
                "API request failed with status code: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<R> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UniFiError::ApiError(
                api_response.meta.msg.unwrap_or_else(|| "Unknown API error".into()),
            ));
        }

        match api_response.data {
            Some(data) => Ok(data),
            None => Err(UniFiError::ApiError("No data returned from API".into())),
        }
    }

    /// Gets the current site ID.
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Gets the guest API interface.
    pub fn guests(&self) -> guest::GuestHandler {
        guest::GuestHandler::new(self.clone())
    }

    // /// Gets the site API interface.
    // pub fn sites(&self) -> SiteApi {
    //     SiteApi::new(self)
    // }

    // /// Gets the voucher API interface.
    // pub fn vouchers(&self) -> VoucherApi {
    //     VoucherApi::new(self)
    // }
}

// Implement Clone for UniFiClient
impl Clone for UniFiClient {
    fn clone(&self) -> Self {
        UniFiClient {
            controller_url: self.controller_url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            site: self.site.clone(),
            verify_ssl: self.verify_ssl,
            timeout: self.timeout,
            user_agent: self.user_agent.clone(),
            http_client: self.http_client.clone(),
            auth_state: self.auth_state.clone(),
        }
    }
}
