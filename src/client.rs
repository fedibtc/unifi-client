use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "default-client")]
use arc_swap::ArcSwap;
use http::Method;
#[cfg(feature = "default-client")]
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::{Client as ReqwestClient, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

use crate::api::guests;
use crate::models::ApiResponse;
use crate::{models, UniFiError, UniFiResult};

// Global default instance
// Initializes to an inert default client; applications should call `initialize()`
// early to replace it with a configured client.
#[cfg(feature = "default-client")]
static UNIFI_CLIENT: Lazy<ArcSwap<UniFiClient>> =
    Lazy::new(|| ArcSwap::from_pointee(UniFiClient::default()));

/// Helper to create a reqwest client builder to ensure consistent configuration.
fn reqwest_builder(timeout: Duration, accept_invalid_certs: bool) -> reqwest::ClientBuilder {
    ReqwestClient::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(accept_invalid_certs)
        .redirect(Policy::none())
        .cookie_store(true)
        .user_agent(concat!("unifi-client/", env!("CARGO_PKG_VERSION")))
}

/// Initializes the global UniFi client instance.
///
/// # Warning
///
/// Call this exactly once at application startup. Calling it again will
/// replace the existing instance used by `instance()`.
///
/// # Arguments
///
/// * `client` - A fully constructed `UniFiClient`.
///
/// # Returns
///
/// - `Arc<UniFiClient>`: The previously configured global client instance. On the first call this
///   will be the inert default instance.
///
/// # Examples
///
/// Basic initialization:
/// ```no_run
/// # use unifi_client::UniFiClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = UniFiClient::builder()
///     .controller_url("https://controller.example:8443")
///     .username("admin")
///     .password("secret")
///     // .accept_invalid_certs(true) // only for lab/test
///     .build()
///     .await?;
/// let _prev = unifi_client::initialize(client);
/// # Ok(())
/// # }
/// ```
///
/// Swapping instances (e.g., tests or hot-reload scenarios):
/// ```no_run
/// # use unifi_client::UniFiClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client_a = UniFiClient::builder()
///     .controller_url("https://a.example:8443")
///     .username("user_a")
///     .password("pass_a")
///     .build()
///     .await?;
/// let _ = unifi_client::initialize(client_a);
///
/// let client_b = UniFiClient::builder()
///     .controller_url("https://b.example:8443")
///     .username("user_b")
///     .password("pass_b")
///     .build()
///     .await?;
/// let previous = unifi_client::initialize(client_b);
/// // `previous` is the prior global client (client_a).
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "default-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "default-client")))]
pub fn initialize(client: UniFiClient) -> Arc<UniFiClient> {
    // Swap in the provided client and return the previous instance.
    UNIFI_CLIENT.swap(Arc::new(client))
}

/// Returns a reference to the global UniFi client instance.
///
/// # Returns
///
/// - `Arc<UniFiClient>`: A thread-safe handle to the current client. If `initialize()` hasn't been
///   called, a default (unauthenticated) client is returned.
///
/// # Examples
///
/// ```no_run
/// # use unifi_client::UniFiError;
/// # #[tokio::main]
/// # async fn main() -> Result<(), UniFiError> {
/// let client = unifi_client::instance();
/// // Use `client` to perform requests...
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "default-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "default-client")))]
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
    password: Option<SecretString>,
    site: Option<String>,
    /// When `true`, TLS certificates are **not** verified (dangerous).
    /// Defaults to `false` (secure-by-default).
    accept_invalid_certs: bool,
    timeout: Option<Duration>,
    http_client: Option<ReqwestClient>,
}

impl UniFiClientBuilder {
    /// Sets the controller URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL of the UniFi controller (e.g., `https://controller.example:8443`).
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn controller_url(mut self, url: impl Into<String>) -> Self {
        self.controller_url = Some(url.into());
        self
    }

    /// Sets the username for authentication.
    ///
    /// # Arguments
    ///
    /// * `username` - The account username.
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password for authentication.
    ///
    /// # Arguments
    ///
    /// * `password` - The account password.
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(SecretString::from(password.into()));
        self
    }

    /// Sets the password from an environment variable.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable containing the password.
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if the environment variable cannot be read.
    pub fn password_from_env(mut self, var_name: &str) -> Self {
        let password = std::env::var(var_name)
            .map_err(|e| format!("Failed to read environment variable '{}': {}", var_name, e))
            .expect("Failed to set password from environment");
        self.password = Some(SecretString::from(password));
        self
    }

    /// Sets the site to use.
    ///
    /// # Arguments
    ///
    /// * `site` - The UniFi site identifier (e.g., `default`).
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn site(mut self, site: impl Into<String>) -> Self {
        self.site = Some(site.into());
        self
    }

    /// Accept invalid/self-signed TLS certificates (dangerous).
    ///
    /// Default is `false` (certificates are verified).
    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    /// Sets the HTTP request timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The overall request timeout duration.
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets a custom reqwest client (e.g., for testing or custom middleware).
    ///
    /// # Arguments
    ///
    /// * `http_client` - A preconfigured `reqwest::Client`.
    ///
    /// # Returns
    ///
    /// - `Self`: The builder for method chaining.
    pub fn http_client(mut self, http_client: ReqwestClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// Builds and authenticates a `UniFiClient`.
    ///
    /// This constructs the HTTP client, detects the controller kind
    /// (UniFi OS vs. Network), configures `api_base_url`, and performs an
    /// initial login.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required fields are missing (username, password, controller URL).
    /// - The controller URL is invalid.
    /// - The HTTP client cannot be created.
    /// - Authentication fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::{UniFiClient, UniFiError};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), UniFiError> {
    /// let client = UniFiClient::builder()
    ///     .controller_url("https://controller.example:8443")
    ///     .username("admin")
    ///     .password("secret")
    ///     // .accept_invalid_certs(true) // only for lab/test
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> UniFiResult<UniFiClient> {
        let site = self.site.unwrap_or_else(|| "default".to_string());

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));

        let username = self
            .username
            .filter(|u| !u.trim().is_empty())
            .ok_or_else(|| UniFiError::ConfigurationError("Username is required".into()))?;

        let password = self
            .password
            .filter(|p| !p.expose_secret().trim().is_empty())
            .ok_or_else(|| UniFiError::ConfigurationError("Password is required".into()))?;

        let controller_url = self
            .controller_url
            .ok_or_else(|| UniFiError::ConfigurationError("Controller URL is required".into()))
            .and_then(|url_str| {
                Url::parse(&url_str).map_err(|e| {
                    UniFiError::ConfigurationError(format!("Invalid controller URL: {e}"))
                })
            })?;

        let http_client = if let Some(custom_client) = self.http_client {
            custom_client
        } else {
            reqwest_builder(timeout, self.accept_invalid_certs)
                .build()
                .map_err(|e| {
                    UniFiError::ConfigurationError(format!("Failed to create HTTP client: {e}"))
                })?
        };

        // Detect controller kind with a lightweight HEAD request to '/'
        let probe_url = controller_url
            .join("/")
            .map_err(|e| UniFiError::UrlParseError(e))?;
        let probe_status = http_client
            .head(probe_url)
            .send()
            .await
            .map(|r| r.status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // UniFi OS servers return 200, UniFi Network controllers return 304 redirects.
        let controller_kind = if probe_status == StatusCode::OK {
            ControllerKind::Os
        } else {
            ControllerKind::Network
        };

        let api_base_url = match controller_kind {
            ControllerKind::Os => controller_url
                .join("/proxy/network")
                .map_err(UniFiError::UrlParseError)?,
            ControllerKind::Network => controller_url.clone(),
        };

        let client = UniFiClient {
            controller_kind,
            controller_url,
            api_base_url,
            username,
            password: Some(password),
            site,
            http_client,
            auth_state: Arc::new(RwLock::new(None)),
        };

        // Perform initial login now that controller kind is known.
        client.login().await?;
        Ok(client)
    }
}

/// Authentication state for the client.
#[derive(Clone, Debug)]
struct AuthState {
    csrf_token: Option<SecretString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControllerKind {
    Network,
    Os,
}

/// The UniFi client for interacting with the UniFi Controller API.
///
/// This client manages authentication, request handling, and provides access
/// to the various API endpoints through dedicated API handlers.
#[derive(Clone)]
pub struct UniFiClient {
    controller_kind: ControllerKind,
    controller_url: Url,
    api_base_url: Url,
    username: String,
    password: Option<SecretString>,
    site: String,
    http_client: ReqwestClient,
    auth_state: Arc<RwLock<Option<AuthState>>>,
}

impl fmt::Debug for UniFiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auth_state_info = match self.auth_state.try_read() {
            Ok(guard) => format!("Some({})", guard.is_some()),
            Err(_) => "Locked".to_string(),
        };

        f.debug_struct("UniFiClient")
            .field("controller_kind", &format!("{:?}", self.controller_kind))
            .field("controller_url", &self.controller_url.as_str())
            .field("api_base_url", &self.api_base_url.as_str())
            .field("username", &self.username)
            .field("password", &self.password)
            .field("site", &self.site)
            .field("auth_state", &auth_state_info)
            .finish()
    }
}

/// Defaults for UniFiClient:
/// - `controller_kind`: `Network`
/// - `controller_url`: `https://example.invalid:8443`
/// - `api_base_url`: `https://example.invalid:8443`
/// - `username`: empty
/// - `password`: `None`
/// - `site`: `default`
/// - `http_client`: reqwest client with `cookie_store(true)` and no redirects
///
/// Note: This Default is inert and intended to be replaced via
/// `UniFiClient::builder().build().await` and `initialize()`. Using the default
/// instance without initialization will produce configuration/authentication errors
/// when attempting to make requests.
impl Default for UniFiClient {
    fn default() -> Self {
        let timeout = Duration::from_secs(30);
        // Secure by default: do NOT accept invalid certs.
        let http_client = reqwest_builder(timeout, false)
            .build()
            .expect("Failed to create default HTTP client");

        UniFiClient {
            controller_kind: ControllerKind::Network,
            controller_url: Url::parse("https://example.invalid:8443")
                .expect("Invalid default URL"),
            api_base_url: Url::parse("https://example.invalid:8443").expect("Invalid default URL"),
            username: String::new(),
            password: None,
            site: "default".to_string(),
            http_client,
            auth_state: Arc::new(RwLock::new(None)),
        }
    }
}

/// # Constructors
impl UniFiClient {
    /// Creates a new `UniFiClientBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// let builder = unifi_client::UniFiClient::builder();
    /// ```
    pub fn builder() -> UniFiClientBuilder {
        UniFiClientBuilder::default()
    }
}

/// # UniFiAPI Handlers
impl UniFiClient {
    /// Gets the current site identifier.
    ///
    /// # Returns
    ///
    /// - `&str`: The configured UniFi site (e.g., `default`).
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Creates a new `guests::GuestHandler` for the Guests API.
    ///
    /// # Returns
    ///
    /// - `guests::GuestHandler`: A typed handler scoped to this client.
    pub fn guests(&self) -> guests::GuestHandler {
        guests::GuestHandler::new(self.clone())
    }
}

/// # UniFi Authentication Methods
impl UniFiClient {
    async fn login(&self) -> UniFiResult<()> {
        // Validate username and password fields once per session initiation.
        if self.username.trim().is_empty() {
            return Err(UniFiError::ConfigurationError(
                "Username is required".into(),
            ));
        }
        let password = self
            .password
            .as_ref()
            .and_then(|p| {
                let s = p.expose_secret();
                (!s.trim().is_empty()).then(|| s.to_owned())
            })
            .ok_or_else(|| UniFiError::ConfigurationError("Password is required".into()))?;

        // Choose login path based on pre-detected controller kind
        let login_path = match self.controller_kind {
            ControllerKind::Os => "/api/auth/login",
            ControllerKind::Network => "/api/login",
        };

        let login_url = self
            .controller_url
            .join(login_path)
            .map_err(|e| UniFiError::UrlParseError(e))?;

        let login_data = models::auth::LoginRequest {
            username: self.username.clone(),
            password,
        };

        let response = self
            .http_client
            .post(login_url)
            .json(&login_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(UniFiError::AuthenticationError(format!(
                "Authentication failed with status code: {}",
                response.status()
            )));
        }

        // Ensure a cookie was set (required for both Network and UniFi OS)
        if response.headers().get("set-cookie").is_none() {
            return Err(UniFiError::AuthenticationError(
                "No cookies received from server".into(),
            ));
        }

        // Capture CSRF token for UniFi OS
        let csrf_token = response
            .headers()
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // For UniFi Network, a JSON body with { meta: { rc: "ok" }, ... } is returned.
        // For UniFi OS, the response is 200 with a body that does not follow that schema.
        if self.controller_kind == ControllerKind::Network {
            let login_response: ApiResponse<Value> = response.json().await?;
            if login_response.meta.rc != "ok" {
                return Err(UniFiError::AuthenticationError(
                    login_response
                        .meta
                        .msg
                        .unwrap_or_else(|| "Unknown error".into()),
                ));
            }
        }

        // Persist auth state (CSRF if present).
        {
            let mut auth_state = self.auth_state.write().await;
            *auth_state = Some(AuthState {
                csrf_token: csrf_token.map(SecretString::from),
            });
        }
        Ok(())
    }

    /// Ensure the client is authenticated.
    async fn ensure_authenticated(&self) -> UniFiResult<()> {
        // If the client is not authenticated, login.
        if self.auth_state.read().await.is_none() {
            return self.login().await;
        }

        let url = self.api_url("/api/self")?;

        match self
            .http_client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await
        {
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
        let auth_state = self.auth_state.read().await;
        let auth_state = auth_state.as_ref().ok_or(UniFiError::NotAuthenticated)?;

        let mut headers = HeaderMap::new();

        if let Some(token) = &auth_state.csrf_token {
            let mut csrf_token = HeaderValue::from_str(token.expose_secret())
                .map_err(|e| UniFiError::ApiError(format!("Invalid CSRF token: {}", e)))?;
            csrf_token.set_sensitive(true);
            headers.insert("x-csrf-token", csrf_token);
        }

        Ok(headers)
    }
}

/// # HTTP Methods
/// A collection of different of HTTP methods to use with UniFiClient's
/// configuration (Authenication, etc.). All of the HTTP methods (`get`, `post`,
/// etc.) perform some amount of pre-processing such as making relative urls
/// absolute, and post processing such as mapping any potential UniFi errors
/// into `Err()` variants, and deserializing the response body.
///
/// This isn't always ideal when working with UniFi's API and as such there is
/// an additional method available, `raw_request()`, that  performs no pre or
/// post processing and directly returns the `http::Response` struct.
///
/// Additionally, `request_json()` is available for endpoints that return the
/// standard `{ meta: { rc: ... }, data: ... }` response. It inspects `meta.rc`
/// and returns the `data` field as a `serde_json::Value`.
impl UniFiClient {
    /// Makes a raw request to the UniFi API.
    ///
    /// # Warning
    ///
    /// This is an advanced API that bypasses the type-safe wrappers.
    /// Use the typed API methods (like `guests()`, `sites()`) when possible.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to use (e.g., `Method::GET`, `Method::POST`).
    /// * `endpoint` - The API endpoint path (e.g., "/api/s/default/stat/sysinfo").
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
    /// # use reqwest::Method;
    /// # use unifi_client::{UniFiClient, UniFiError};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), UniFiError> {
    /// // For production use, get the password from a secure location.
    /// let unifi_client = UniFiClient::builder()
    ///     .controller_url("https://your-controller-url:8443")
    ///     .username("your_username")
    ///     .password("your_password")
    ///     .build()
    ///     .await?;
    ///
    /// // Get system status with a raw request.
    /// let status = unifi_client
    ///     .raw_request(Method::GET, "/api/s/default/stat/sysinfo", None::<()>)
    ///     .await?;
    ///
    /// println!("System info: {:?}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn raw_request<T>(
        &self,
        method: http::Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<reqwest::Response>
    where
        T: Serialize,
    {
        let response = self.send_http(method, endpoint, body).await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(UniFiError::NotAuthenticated);
        }

        Ok(response)
    }

    /// Makes a raw request and returns the parsed JSON body.
    ///
    /// For endpoints that follow UniFi's standard response shape:
    /// `{ meta: { rc: "ok" }, data: ... }`. This method checks `meta.rc`
    /// and returns the `data` field as a `serde_json::Value`.
    ///
    /// # Arguments
    ///
    /// - `method`: The HTTP method to use.
    /// - `endpoint`: The API endpoint path (e.g., `/api/s/default/stat/sysinfo`).
    /// - `body`: Optional request body.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The request fails or authentication is invalid.
    /// - The response body cannot be parsed.
    /// - `meta.rc` is not `"ok"`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reqwest::Method;
    /// # use unifi_client::{UniFiClient, UniFiError};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), UniFiError> {
    /// // For production use, get the password from a secure location.
    /// let unifi_client = UniFiClient::builder()
    ///     .controller_url("https://your-controller-url:8443")
    ///     .username("your_username")
    ///     .password("your_password")
    ///     .build()
    ///     .await?;
    /// let json = unifi_client
    ///     .request_json::<()>(Method::GET, "/api/s/default/stat/health", None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_json<T>(
        &self,
        method: http::Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<serde_json::Value>
    where
        T: Serialize,
    {
        let response = self.raw_request(method, endpoint, body).await?;

        if !response.status().is_success() {
            return Err(UniFiError::ApiError(format!(
                "API request failed with status code: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<serde_json::Value> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UniFiError::ApiError(
                api_response
                    .meta
                    .msg
                    .unwrap_or_else(|| "Unknown API error".into()),
            ));
        }

        Ok(api_response.data.unwrap_or(serde_json::Value::Null))
    }

    /// Sends a GET request and parses the standard UniFi API response.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Query/body type to serialize.
    /// - `R`: Response type to deserialize into.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The API endpoint path.
    /// * `params` - Optional parameters sent as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, authentication is invalid, or
    /// the response cannot be deserialized into `R`.
    pub async fn get<T, R>(&self, endpoint: &str, params: Option<T>) -> UniFiResult<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let value: serde_json::Value = self.request_json(Method::GET, endpoint, params).await?;

        if value.is_null() {
            return Err(UniFiError::ApiError("No data returned from API".into()));
        }

        let data = serde_json::from_value::<R>(value)?;
        Ok(data)
    }

    /// Sends a POST request and parses the standard UniFi API response.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Request body type to serialize.
    /// - `R`: Response type to deserialize into.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The API endpoint path.
    /// * `body` - Optional JSON body.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, authentication is invalid, or
    /// the response cannot be deserialized into `R`.
    pub async fn post<T, R>(&self, endpoint: &str, body: Option<T>) -> UniFiResult<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let value: serde_json::Value = self.request_json(Method::POST, endpoint, body).await?;

        if value.is_null() {
            return Err(UniFiError::ApiError("No data returned from API".into()));
        }

        let data = serde_json::from_value::<R>(value)?;
        Ok(data)
    }

    /// Core HTTP sender used by raw_request() and request().
    ///
    /// - Ensures authentication
    /// - Builds the URL from `api_base_url` + endpoint
    /// - Applies auth and CSRF headers
    /// - Sends the request and updates CSRF token from response headers (if present)
    async fn send_http<T>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<reqwest::Response>
    where
        T: Serialize,
    {
        self.ensure_authenticated().await?;

        let url = self.api_url(endpoint)?;
        let mut request = self.http_client.request(method, url);

        request = request.headers(self.get_auth_headers().await?);

        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;

        // Refresh CSRF token if the server rotated it
        if let Some(updated) = response
            .headers()
            .get("x-updated-csrf-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
        {
            let mut auth_state = self.auth_state.write().await;
            if let Some(state) = auth_state.as_mut() {
                state.csrf_token = Some(SecretString::from(updated));
            }
        }

        Ok(response)
    }
}

/// # Utility Methods
impl UniFiClient {
    // Build the URL for an API endpoint using path segments to avoid trailing slash issues.
    fn api_url(&self, endpoint: &str) -> UniFiResult<Url> {
        let mut url = self.api_base_url.clone();
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|_| UniFiError::ConfigurationError("Base URL cannot be a base".into()))?;
            // Remove a trailing empty segment (e.g., ends with '/') to avoid creating '//'.
            path_segments.pop_if_empty();
            for s in endpoint.split('/').filter(|s| !s.is_empty()) {
                path_segments.push(s);
            }
        }
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client_with_api_base_url(api_base_url: &str, kind: ControllerKind) -> UniFiClient {
        UniFiClient {
            controller_kind: kind,
            controller_url: Url::parse("https://example.com/").unwrap(),
            api_base_url: Url::parse(api_base_url).unwrap(),
            username: "user".into(),
            password: Some(SecretString::from("pass")),
            site: "default".into(),
            http_client: reqwest::Client::new(),
            auth_state: Arc::new(RwLock::new(None)),
        }
    }

    #[test]
    fn api_url_network_preserves_base_and_appends_segments() {
        let client = make_client_with_api_base_url("https://example.com/", ControllerKind::Network);

        // Leading slash
        let url = client.api_url("/api/s/default/stat/guest").unwrap();
        assert_eq!(url.as_str(), "https://example.com/api/s/default/stat/guest");

        // No leading slash
        let url = client.api_url("api/self").unwrap();
        assert_eq!(url.as_str(), "https://example.com/api/self");
    }

    #[test]
    fn api_url_unifi_os_keeps_proxy_network_prefix() {
        let client =
            make_client_with_api_base_url("https://example.com/proxy/network/", ControllerKind::Os);

        // Leading slash
        let url = client.api_url("/api/s/default/stat/guest").unwrap();
        assert_eq!(
            url.as_str(),
            "https://example.com/proxy/network/api/s/default/stat/guest"
        );

        // No leading slash
        let url = client.api_url("api/self").unwrap();
        assert_eq!(url.as_str(), "https://example.com/proxy/network/api/self");
    }

    #[test]
    fn api_url_normalizes_redundant_slashes() {
        let client =
            make_client_with_api_base_url("https://example.com/proxy/network/", ControllerKind::Os);

        // Multiple redundant slashes should be normalized by segment push
        let url = client
            .api_url("///api//s///default//stat///guest//")
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://example.com/proxy/network/api/s/default/stat/guest"
        );
    }
}
