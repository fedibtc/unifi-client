use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "default-client")]
use arc_swap::ArcSwap;
use http::Method;
#[cfg(feature = "default-client")]
use once_cell::sync::Lazy;
use reqwest::header::HeaderValue;
use reqwest::redirect::Policy;
use reqwest::{Client as ReqwestClient, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
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

const HEADER_CSRF_TOKEN: &str = "x-csrf-token";
const HEADER_UPDATED_CSRF_TOKEN: &str = "x-updated-csrf-token";

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
/// - `client` - A fully constructed `UniFiClient`.
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
        self.password = Some(SecretString::from(password.into()));
        self
    }

    /// Sets the password from an environment variable.
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

    /// Sets the site name (e.g., `default`, `qc4lt5rs`) to use.
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
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets a custom reqwest client (e.g., for testing or custom middleware).
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
    /// # Example
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
            auth: Arc::new(AuthState::new()),
        };

        // Perform initial login to fail fast if authentication fails.
        client.login().await?;
        Ok(client)
    }
}

/// Authentication state for the client.
#[derive(Debug)]
struct AuthState {
    /// Cross-Site Request Forgery (CSRF) token.
    /// - Present on UniFi OS
    /// - `None` for classic Network controllers
    csrf_token: RwLock<Option<SecretString>>,

    /// The mutex to ensure only one thread attempts to re-login at a time.
    reauth_lock: Mutex<()>,

    /// The epoch/generation counter to prevent the "thundering herd".
    /// Incremented on successful login to signal a new session.
    epoch: AtomicUsize,
}

impl AuthState {
    fn new() -> Self {
        Self {
            csrf_token: RwLock::new(None),
            reauth_lock: Mutex::new(()),
            epoch: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn epoch(&self) -> usize {
        self.epoch.load(Ordering::Acquire)
    }

    fn is_authenticated(&self) -> bool {
        self.epoch() != 0
    }

    /// Rotate CSRF token mid-session when server provides a new value (only present for UniFi OS).
    async fn rotate_csrf(&self, token: impl Into<SecretString>) {
        // Convert to owned secret **before** any await to avoid borrowing across await.
        let token: SecretString = token.into();
        let mut w = self.csrf_token.write().await;
        *w = Some(token);
    }

    /// Apply the results of a successful authentication:
    /// - set/clear the CSRF token (OS: Some, Network: None)
    /// - advance the auth epoch
    /// Returns the new epoch.
    async fn establish_session<T>(&self, csrf_token: Option<T>) -> usize
    where
        T: Into<SecretString>,
    {
        {
            let mut w = self.csrf_token.write().await;
            *w = csrf_token.map(Into::into);
        }

        // Advance the auth generation counter and return the new value.
        // Uses `AcqRel` so writes that happen before bumping (e.g., CSRF update)
        // are visible to tasks that observe the new epoch with `Acquire` loads.
        self.epoch.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Deduplicate concurrent re-auth attempts.
    ///
    /// Semantics:
    /// - Exactly one caller ("leader") runs `login_fn` while holding the guard.
    /// - All other callers block on the guard; once it unlocks, they observe the advanced epoch and
    ///   skip running `login_fn`.
    ///
    /// Returns:
    /// - `Ok(true)`  if this call performed the login (was the leader).
    /// - `Ok(false)` if another task already completed re-authentication while we waited.
    async fn dedupe_reauthentication<F, Fut>(&self, login_fn: F) -> UniFiResult<bool>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = UniFiResult<()>>,
    {
        // Get the current epoch (or 0 if not authenticated).
        let epoch_before = self.epoch();

        // Acquire the lock to serialize authentication attempts.
        let _guard = self.reauth_lock.lock().await;

        // Check if another thread already re-authenticated while we waited.
        if self.epoch() == epoch_before {
            // Still stale: we are the leader; perform re-authentication.
            login_fn().await?;
            return Ok(true);
        }

        // Another thread already re-authenticated while we waited.
        Ok(false)
    }
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
    auth: Arc<AuthState>,
}

impl fmt::Debug for UniFiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let epoch = self.auth.epoch.load(Ordering::Relaxed);
        let csrf_present = self
            .auth
            .csrf_token
            .try_read()
            .map(|g| g.is_some())
            .unwrap_or(false);

        f.debug_struct("UniFiClient")
            .field("controller_kind", &format!("{:?}", self.controller_kind))
            .field("controller_url", &self.controller_url.as_str())
            .field("api_base_url", &self.api_base_url.as_str())
            .field("username", &self.username)
            .field("password", &self.password)
            .field("site", &self.site)
            .field("auth_epoch", &epoch)
            .field("csrf_present", &csrf_present)
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
            auth: Arc::new(AuthState::new()),
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

        // Capture CSRF token (UniFi OS) if present
        let csrf_token = response
            .headers()
            .get(HEADER_CSRF_TOKEN)
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

        // Persist auth state.
        // UniFi Network uses an HTTP cookie (automatically handled by reqwest cookie store)
        // UniFi OS uses a CSRF token, which is stored in the auth state.
        self.auth.establish_session(csrf_token).await;

        Ok(())
    }

    // Helper to get authentication headers
    async fn csrf_header_value(&self) -> UniFiResult<Option<HeaderValue>> {
        if !self.auth.is_authenticated() {
            // Unauthenticated client asked for CSRF header.
            return Err(UniFiError::NotAuthenticated);
        }

        if let Some(token) = self.auth.csrf_token.read().await.as_ref() {
            let mut hv = HeaderValue::from_str(token.expose_secret())
                .map_err(|e| UniFiError::ApiError(format!("Invalid CSRF token: {e}")))?;
            hv.set_sensitive(true);
            Ok(Some(hv))
        } else {
            Ok(None)
        }
    }
}

/// # HTTP Methods
/// This impl block provides typed helpers (`get`, `post`) and lower-level
/// methods (`request`, `request_json`) that respect the client's authentication
/// and CSRF handling.
impl UniFiClient {
    /// Sends a GET request and parses the standard UniFi API response.
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

    /// Makes a request and returns the parsed JSON body.
    ///
    /// For endpoints that follow UniFi's standard response shape:
    /// `{ meta: { rc: "ok" }, data: ... }`. This method checks `meta.rc`
    /// and returns the `data` field as a `serde_json::Value`.
    ///
    /// # Example
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
    ///     .request_json(Method::GET, "/api/s/default/stat/health", None::<()>)
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
        let response = self.request(method, endpoint, body).await?;

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

    /// Makes an HTTP request to the UniFi API and returns the raw response.
    ///
    /// Behavior:
    /// - Builds the URL from `api_base_url` and `endpoint`
    /// - Adds UniFi OS CSRF header if present
    /// - Sends the request
    /// - Rotates CSRF if the server provides `x-updated-csrf-token`
    /// - On 401 (both kinds) or 403 (OS), performs a single-flight re-login and retries once
    ///
    /// This is the low-level escape hatch; prefer typed methods when available.
    ///
    /// # Example
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
    /// // Get system status.
    /// let status = unifi_client
    ///     .request(Method::GET, "/api/s/default/stat/sysinfo", None::<()>)
    ///     .await?;
    ///
    /// println!("System info: {:?}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request<T>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UniFiResult<reqwest::Response>
    where
        T: Serialize,
    {
        debug_assert!(
            self.auth.is_authenticated(),
            "Client must be constructed via `build()` which performs an initial login"
        );

        let mut retries = 0u8;

        loop {
            let url = self.api_url(endpoint)?;
            let mut request = self.http_client.request(method.clone().into(), url);

            if let Some(ref data) = body {
                request = request.json(data);
            }

            // Add CSRF header if present (UniFi OS only)
            if self.controller_kind == ControllerKind::Os {
                if let Some(csrf) = self.csrf_header_value().await? {
                    request = request.header(HEADER_CSRF_TOKEN, csrf);
                }
            }

            let response = request.send().await?;

            // Always rotate CSRF token first if present (UniFi OS can rotate on success or error).
            if let Some(updated_token) = response
                .headers()
                .get(HEADER_UPDATED_CSRF_TOKEN)
                .and_then(|v| v.to_str().ok())
            {
                self.auth.rotate_csrf(updated_token).await;
            }

            // Retry if the request failed due to authentication or authorization.
            let should_retry = matches!(
                (response.status(), self.controller_kind),
                (StatusCode::UNAUTHORIZED, _) | (StatusCode::FORBIDDEN, ControllerKind::Os)
            );
            if !should_retry {
                return Ok(response);
            }

            if retries >= 1 {
                return Err(UniFiError::NotAuthenticated);
            }

            // Ensure only one thread attempts to re-authenticate to avoid stampedes.
            self.auth
                .dedupe_reauthentication(|| async { self.login().await })
                .await?;

            retries += 1;
            // Retry the request once with refreshed auth.
        }
    }
}

/// # Utility Methods
impl UniFiClient {
    // Build the URL for an API endpoint using path segments to avoid trailing slash issues.
    fn api_url(&self, endpoint: &str) -> UniFiResult<Url> {
        if endpoint.contains(['?', '#']) {
            return Err(UniFiError::InvalidEndpoint(format!(
                "endpoint must not include query or fragment: {endpoint}"
            )));
        }

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
            auth: Arc::new(AuthState::new()),
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

    #[test]
    fn api_url_rejects_query_or_fragment() {
        let client = make_client_with_api_base_url("https://example.com/", ControllerKind::Network);

        // Query string should be rejected
        let err = client.api_url("/api/self?foo=bar").unwrap_err();
        match err {
            UniFiError::InvalidEndpoint(msg) => {
                assert!(msg.contains("query or fragment"), "msg was: {msg}");
            }
            other => panic!("Expected InvalidEndpoint, got {other:?}"),
        }

        // Fragment should be rejected
        let err = client.api_url("/api/self#frag").unwrap_err();
        match err {
            UniFiError::InvalidEndpoint(msg) => {
                assert!(msg.contains("query or fragment"), "msg was: {msg}");
            }
            other => panic!("Expected InvalidEndpoint, got {other:?}"),
        }
    }
}
