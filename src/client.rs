use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::{Client as ReqwestClient, Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

use crate::api::guests;
use crate::models::ApiResponse;
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
    password: Option<SecretString>,
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
        self.password = Some(SecretString::from(password.into()));
        self
    }

    /// Sets the password from an environment variable.
    pub fn password_from_env(mut self, var_name: &str) -> Self {
        let password = std::env::var(var_name)
            .map_err(|e| format!("Failed to read environment variable '{}': {}", var_name, e))
            .expect("Failed to set password from environment");
        self.password = Some(SecretString::from(password));
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

        let password = self
            .password
            .ok_or_else(|| UniFiError::ConfigurationError("Password is required".into()))?;

        let username = self
            .username
            .ok_or_else(|| UniFiError::ConfigurationError("Username is required".into()))?;

        let controller_url = self
            .controller_url
            .ok_or_else(|| UniFiError::ConfigurationError("Controller URL is required".into()))
            .and_then(|url_str| {
                Url::parse(&url_str).map_err(|e| {
                    UniFiError::ConfigurationError(format!("Invalid controller URL: {e}"))
                })
            })?;

        let user_agent = self
            .user_agent
            .as_deref()
            .unwrap_or(concat!("unifi-client/", env!("CARGO_PKG_VERSION")));

        let http_client = if let Some(custom_client) = self.http_client {
            custom_client
        } else {
            ReqwestClient::builder()
                .timeout(timeout)
                .danger_accept_invalid_certs(!self.verify_ssl)
                .redirect(Policy::none())
                .cookie_store(true)
                .user_agent(user_agent)
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
            verify_ssl: self.verify_ssl,
            timeout,
            user_agent: self.user_agent,
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
    verify_ssl: bool,
    timeout: Duration,
    user_agent: Option<String>,
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
            .field("verify_ssl", &self.verify_ssl)
            .field("timeout", &self.timeout)
            .field("user_agent", &self.user_agent)
            .field("auth_state", &auth_state_info)
            .finish()
    }
}

/// Defaults for UniFiClient:
/// - `controller_kind`: `Network`
/// - `controller_url`: `https://localhost:8443`
/// - `api_base_url`: `https://localhost:8443`
/// - `username`: `admin`
/// - `password`: `admin`
/// - `site`: `default`
/// - `verify_ssl`: `false`
/// - `timeout`: `30 seconds`
/// - `http_client`: bare `reqwest::Client::new()` (no cookie store configured)
///
/// Note: This Default is intended for local testing and debugging. For real use,
/// construct a client via `UniFiClient::builder()`, which configures the HTTP
/// client, detects controller kind, sets `api_base_url`, and performs the initial
/// login.
impl Default for UniFiClient {
    fn default() -> Self {
        UniFiClient {
            controller_kind: ControllerKind::Network,
            controller_url: Url::parse("https://localhost:8443")
                .expect("Failed to parse default URL"),
            api_base_url: Url::parse("https://localhost:8443")
                .expect("Failed to parse default URL"),
            username: "admin".to_string(),
            password: Some(SecretString::from("admin")),
            site: "default".to_string(),
            verify_ssl: false,
            timeout: Duration::from_secs(30),
            user_agent: Some(concat!("unifi-client/", env!("CARGO_PKG_VERSION")).to_string()),
            http_client: reqwest::Client::new(),
            auth_state: Arc::new(RwLock::new(None)),
        }
    }
}

/// # Constructors
impl UniFiClient {
    pub fn builder() -> UniFiClientBuilder {
        UniFiClientBuilder::default()
    }
}

/// # UniFiAPI Handlers
impl UniFiClient {
    /// Gets the current site ID.
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Creates a new [`guests::GuestHandler`] for accessing information from UniFi's Guest API.
    pub fn guests(&self) -> guests::GuestHandler {
        guests::GuestHandler::new(self.clone())
    }
}

/// # UniFi Authentication Methods
impl UniFiClient {
    async fn login(&self) -> UniFiResult<()> {
        let password = match &self.password {
            Some(pwd) => pwd.expose_secret().to_string(),
            None => {
                return Err(UniFiError::AuthenticationError(
                    "No password provided for authentication".into(),
                ))
            }
        };

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
        // Check if essential fields are configured *before* trying to use them.
        if self.username.is_empty() {
            return Err(UniFiError::ConfigurationError(
                "Username is required".into(),
            ));
        }
        if self.controller_url.as_str().is_empty() {
            return Err(UniFiError::ConfigurationError(
                "Controller URL is required".into(),
            ));
        }
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
/// This isn't always ideal when working with UniFi's API and as such there are
/// additional methods available prefixed with `_` (e.g.  `_get`, `_post`,
/// etc.) that perform no pre or post processing and directly return the
/// `http::Response` struct.
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
    /// * `method` - The HTTP method to use (e.g., "GET", "POST").
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

        let url = self.api_url(endpoint)?;

        let mut request = self.http_client.request(
            Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET),
            url,
        );

        request = request.headers(self.get_auth_headers().await?);

        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;
        let api_response: ApiResponse<Value> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UniFiError::ApiError(
                api_response
                    .meta
                    .msg
                    .unwrap_or_else(|| "Unknown API error".into()),
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

        let url = self.api_url(endpoint)?;

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
                api_response
                    .meta
                    .msg
                    .unwrap_or_else(|| "Unknown API error".into()),
            ));
        }

        match api_response.data {
            Some(data) => Ok(data),
            None => Err(UniFiError::ApiError("No data returned from API".into())),
        }
    }
}

impl UniFiClient {
    /// Build the absolute URL for an API endpoint using path segments.
    ///
    /// This approach avoids trailing slash pitfalls.
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
            verify_ssl: true,
            timeout: Duration::from_secs(30),
            user_agent: Some("unifi-client/test".into()),
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
