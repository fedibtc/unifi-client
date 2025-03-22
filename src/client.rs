use std::fmt;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
use reqwest::{Client as ReqwestClient, Method, StatusCode};
use rpassword::prompt_password;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::{
    ApiResponse, EmptyResponse, GuestApi, LoginRequest, SiteApi, UnifiError, UnifiResult,
    VoucherApi,
};

/// Configuration for the UniFi client.
///
/// This structure holds all settings needed to connect to a UniFi Controller,
/// including connection parameters, authentication details, and request
/// options.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// The complete URL to the UniFi controller (e.g., "https://unifi.example.com:8443").
    pub controller_url: Url,

    /// Username for authentication.
    pub username: String,

    /// Optional password (if not provided, will prompt at login).
    pub password: Option<String>,

    /// The UniFi site to use (defaults to "default").
    pub site: String,

    /// Whether to verify SSL certificates.
    pub verify_ssl: bool,

    /// HTTP request timeout.
    pub timeout: Duration,

    /// Custom user agent string.
    pub user_agent: Option<String>,
}

impl ClientConfig {
    /// Creates a new client configuration builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use unifi_client::ClientConfig;
    ///
    /// let config = ClientConfig::builder()
    ///     .controller_url("https://unifi.example.com:8443")
    ///     .username("admin")
    ///     .password("secret")
    ///     .site("default")
    ///     .verify_ssl(true)
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .expect("Failed to build client configuration");
    /// ```
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::default()
    }
}

/// Builder for client configuration.
///
/// This builder provides a fluent API for creating UniFi client configurations
/// with validation at build time.
#[derive(Default)]
pub struct ClientConfigBuilder {
    controller_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    site: Option<String>,
    verify_ssl: bool,
    timeout: Option<Duration>,
    user_agent: Option<String>,
}

impl ClientConfigBuilder {
    /// Sets the controller URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The complete URL to the UniFi controller (e.g., "https://unifi.example.com:8443")
    pub fn controller_url(mut self, url: &str) -> Self {
        self.controller_url = Some(url.to_string());
        self
    }

    /// Sets the username for authentication.
    ///
    /// # Arguments
    ///
    /// * `username` - The username to authenticate with
    pub fn username(mut self, username: &str) -> Self {
        self.username = Some(username.to_string());
        self
    }

    /// Sets the password for authentication.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to authenticate with
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    /// Sets the site to use.
    ///
    /// # Arguments
    ///
    /// * `site` - The UniFi site name to use (defaults to "default" if not set)
    pub fn site(mut self, site: &str) -> Self {
        self.site = Some(site.to_string());
        self
    }

    /// Sets whether to verify SSL certificates.
    ///
    /// # Arguments
    ///
    /// * `verify` - Whether to verify SSL certificates (true) or accept invalid
    ///   certs (false)
    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.verify_ssl = verify;
        self
    }

    /// Sets the HTTP request timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The maximum duration to wait for HTTP requests
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets a custom user agent string.
    ///
    /// # Arguments
    ///
    /// * `user_agent` - The user agent string to use for HTTP requests
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// Builds the client configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid:
    /// - Controller URL is required and must be a valid URL
    /// - Username is required
    pub fn build(self) -> UnifiResult<ClientConfig> {
        let controller_url = self
            .controller_url
            .ok_or_else(|| UnifiError::ConfigurationError("Controller URL is required".into()))?;

        let url = Url::parse(&controller_url).map_err(|e| {
            UnifiError::ConfigurationError(format!("Invalid controller URL: {}", e))
        })?;

        let username = self
            .username
            .ok_or_else(|| UnifiError::ConfigurationError("Username is required".into()))?;

        let site = self.site.unwrap_or_else(|| "default".to_string());

        let timeout = self.timeout.unwrap_or_else(|| Duration::from_secs(30));

        Ok(ClientConfig {
            controller_url: url,
            username,
            password: self.password,
            site,
            verify_ssl: self.verify_ssl,
            timeout,
            user_agent: self.user_agent,
        })
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
pub struct UnifiClient {
    pub(crate) config: ClientConfig,
    http_client: ReqwestClient,
    auth_state: Option<AuthState>,
}

impl fmt::Debug for UnifiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnifiClient")
            .field("config", &self.config)
            .field("auth_state", &self.auth_state.is_some())
            .finish()
    }
}

impl UnifiClient {
    /// Creates a new UniFi client with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for connecting to the UniFi Controller
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use unifi_client::{ClientConfig, UnifiClient};
    ///
    /// let config = ClientConfig::builder()
    ///     .controller_url("https://unifi.example.com:8443")
    ///     .username("admin")
    ///     .password("secret")
    ///     .build()
    ///     .expect("Failed to build client config");
    ///
    /// let client = UnifiClient::new(config);
    /// ```
    pub fn new(config: ClientConfig) -> Self {
        let http_client = ReqwestClient::builder()
            .timeout(config.timeout)
            .danger_accept_invalid_certs(!config.verify_ssl)
            .user_agent(config.user_agent.as_deref().unwrap_or("unifi-client/0.1.0"))
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            auth_state: None,
        }
    }

    /// Logs in to the UniFi controller.
    ///
    /// # Arguments
    ///
    /// * `password` - Optional password to use for authentication. If None, the
    ///   password from the configuration will be used. If that is also None,
    ///   the user will be prompted to enter a password.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, typically due to:
    /// - Invalid credentials
    /// - Connection issues
    /// - Server errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), unifi_client::UnifiError> {
    /// # use unifi_client::{ClientConfig, UnifiClient};
    /// # let config = ClientConfig::builder()
    /// #    .controller_url("https://unifi.example.com")
    /// #    .username("admin")
    /// #    .build()?;
    /// let mut client = UnifiClient::new(config);
    ///
    /// // Login with explicit password
    /// client.login(Some("password123".to_string())).await?;
    ///
    /// // Or use password from config or prompt
    /// client.login(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login(&mut self, password: Option<String>) -> UnifiResult<()> {
        let password = match (password, &self.config.password) {
            (Some(pwd), _) => pwd,
            (None, Some(pwd)) => pwd.clone(),
            (None, None) => prompt_password("Enter UniFi controller password: ").map_err(|e| {
                UnifiError::AuthenticationError(format!("Failed to read password: {}", e))
            })?,
        };

        let login_url = self
            .config
            .controller_url
            .join("/api/login")
            .map_err(|e| UnifiError::UrlParseError(e))?;

        let login_data = LoginRequest {
            username: self.config.username.clone(),
            password,
        };

        let response = self
            .http_client
            .post(login_url)
            .json(&login_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(UnifiError::AuthenticationError(format!(
                "Authentication failed with status code: {}",
                response.status()
            )));
        }

        // Extract cookies and CSRF token before consuming the response
        let cookie_header = response
            .headers()
            .get("set-cookie")
            .ok_or_else(|| {
                UnifiError::AuthenticationError("No cookies received from server".into())
            })?
            .to_str()
            .map_err(|e| UnifiError::AuthenticationError(format!("Invalid cookie header: {}", e)))?
            .to_string();

        // Extract CSRF token if present
        let csrf_token = response
            .headers()
            .get("x-csrf-token")
            .map(|v| v.to_str().unwrap_or_default().to_string());

        // Parse response body
        let login_response: ApiResponse<Vec<EmptyResponse>> = response.json().await?;

        if login_response.meta.rc != "ok" {
            return Err(UnifiError::AuthenticationError(
                login_response
                    .meta
                    .msg
                    .unwrap_or_else(|| "Unknown error".into()),
            ));
        }

        self.auth_state = Some(AuthState {
            cookies: cookie_header,
            csrf_token,
        });

        Ok(())
    }

    /// Ensure the client is authenticated by making a lightweight API call.
    /// If the call fails with an authentication error, re-authenticate.
    async fn ensure_authenticated(&mut self) -> UnifiResult<()> {
        if self.auth_state.is_none() {
            return Err(UnifiError::NotAuthenticated);
        }

        // Try to access a lightweight endpoint to verify authentication
        let url = self
            .config
            .controller_url
            .join("/api/self")
            .map_err(|e| UnifiError::UrlParseError(e))?;

        // Make the request but handle authentication errors specially
        match self.http_client
            .get(url)
            .headers(self.get_auth_headers()?)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    // Session is still valid
                    return Ok(());
                } else if response.status() == StatusCode::UNAUTHORIZED {
                    // Session expired, need to re-authenticate
                    self.login(None).await?;
                } else {
                    // Some other API error, but authentication might still be valid
                }
            },
            Err(_e) => {
                // Could be a network error or other issue
                // We'll try to re-authenticate just in case
                self.login(None).await?;
            }
        }

        Ok(())
    }

    // Helper to get authentication headers
    fn get_auth_headers(&self) -> UnifiResult<HeaderMap> {
        let auth_state = self.auth_state.as_ref()
            .ok_or(UnifiError::NotAuthenticated)?;
        
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&auth_state.cookies)
                .map_err(|e| UnifiError::ApiError(format!("Invalid cookie header: {}", e)))?,
        );

        if let Some(token) = &auth_state.csrf_token {
            headers.insert(
                "x-csrf-token",
                HeaderValue::from_str(token)
                    .map_err(|e| UnifiError::ApiError(format!("Invalid CSRF token: {}", e)))?,
            );
        }
        
        Ok(headers)
    }

    /// Makes a raw request to the UniFi API.
    ///
    /// # Warning
    ///
    /// This is an advanced API that bypasses the type-safe wrappers.
    /// Use the typed API methods (like `vouchers()`, `sites()`) when possible.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method to use
    /// * `endpoint` - The API endpoint path
    /// * `body` - Optional request body
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is not valid or expired
    /// - The request fails due to network issues
    /// - The API returns an error response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), unifi_client::UnifiError> {
    /// # use unifi_client::{ClientConfig, UnifiClient};
    /// # let config = ClientConfig::builder()
    /// #    .controller_url("https://unifi.example.com")
    /// #    .username("admin")
    /// #    .password("password")
    /// #    .build()?;
    /// # let mut client = UnifiClient::new(config);
    /// # client.login(None).await?;
    /// // Get system status with a raw request
    /// let status = client
    ///     .raw_request("GET", "/api/s/default/stat/sysinfo", None::<()>)
    ///     .await?;
    /// println!("System info: {}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn raw_request<T>(
        &mut self,
        method: &str,
        endpoint: &str,
        body: Option<T>,
    ) -> UnifiResult<Value>
    where
        T: Serialize,
    {
        self.ensure_authenticated().await?;

        let auth_state = self.auth_state.as_ref().unwrap();
        let url = self.config.controller_url.join(endpoint)?;

        let mut request = self.http_client.request(
            Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET),
            url,
        );

        // Add cookies and CSRF token
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&auth_state.cookies)
                .map_err(|e| UnifiError::ApiError(format!("Invalid cookie header: {}", e)))?,
        );

        if let Some(token) = &auth_state.csrf_token {
            headers.insert(
                "x-csrf-token",
                HeaderValue::from_str(token)
                    .map_err(|e| UnifiError::ApiError(format!("Invalid CSRF token: {}", e)))?,
            );
        }

        request = request.headers(headers);

        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;
        let api_response: ApiResponse<Value> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UnifiError::ApiError(
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
        &mut self,
        method: Method,
        endpoint: &str,
        body: Option<T>,
    ) -> UnifiResult<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.ensure_authenticated().await?;

        let auth_state = self.auth_state.as_ref().unwrap();

        let url = self
            .config
            .controller_url
            .join(endpoint)
            .map_err(|e| UnifiError::UrlParseError(e))?;

        let mut request = self.http_client.request(method, url);

        // Add cookies
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&auth_state.cookies)
                .map_err(|e| UnifiError::ApiError(format!("Invalid cookie header: {}", e)))?,
        );

        // Add CSRF token if available
        if let Some(token) = &auth_state.csrf_token {
            headers.insert(
                "x-csrf-token",
                HeaderValue::from_str(token)
                    .map_err(|e| UnifiError::ApiError(format!("Invalid CSRF token: {}", e)))?,
            );
        }

        request = request.headers(headers);

        // Add JSON body if provided
        if let Some(data) = body {
            request = request.json(&data).header(CONTENT_TYPE, "application/json");
        }

        let response = request.send().await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(UnifiError::NotAuthenticated);
        }

        if !response.status().is_success() {
            return Err(UnifiError::ApiError(format!(
                "API request failed with status code: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse<R> = response.json().await?;

        if api_response.meta.rc != "ok" {
            return Err(UnifiError::ApiError(
                api_response
                    .meta
                    .msg
                    .unwrap_or_else(|| "Unknown API error".into()),
            ));
        }

        match api_response.data {
            Some(data) => Ok(data),
            None => Err(UnifiError::ApiError("No data returned from API".into())),
        }
    }

    /// Gets the current site ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::{ClientConfig, UnifiClient};
    /// # let config = ClientConfig::builder()
    /// #    .controller_url("https://unifi.example.com")
    /// #    .username("admin")
    /// #    .site("my-site")
    /// #    .build()
    /// #    .unwrap();
    /// let client = UnifiClient::new(config);
    /// assert_eq!(client.site(), "my-site");
    /// ```
    pub fn site(&self) -> &str {
        &self.config.site
    }

    /// Gets the guest API interface.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), unifi_client::UnifiError> {
    /// # use unifi_client::{ClientConfig, UnifiClient, GuestConfig};
    /// # let mut client = UnifiClient::new(ClientConfig::builder().controller_url("https://example.com").username("admin").build()?);
    /// # client.login(None).await?;
    /// let guests_api = client.guests();
    ///
    /// // Now use the guests API
    /// let config = GuestConfig::builder()
    ///     .mac("00:11:22:33:44:55")
    ///     .duration(60)
    ///     .build()?;
    /// let guest = guests_api.authorize(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn guests(&self) -> GuestApi {
        GuestApi::new(self)
    }

    /// Gets the site API interface.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), unifi_client::UnifiError> {
    /// # use unifi_client::{ClientConfig, UnifiClient};
    /// # let mut client = UnifiClient::new(ClientConfig::builder().controller_url("https://example.com").username("admin").build()?);
    /// # client.login(None).await?;
    /// let sites_api = client.sites();
    ///
    /// // Now use the sites API
    /// let all_sites = sites_api.list().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn sites(&self) -> SiteApi {
        SiteApi::new(self)
    }

    /// Gets the voucher API interface.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), unifi_client::UnifiError> {
    /// # use unifi_client::{ClientConfig, UnifiClient, VoucherConfig};
    /// # let mut client = UnifiClient::new(ClientConfig::builder().controller_url("https://example.com").username("admin").build()?);
    /// # client.login(None).await?;
    /// let vouchers_api = client.vouchers();
    ///
    /// // Now use the vouchers API
    /// let config = VoucherConfig::builder()
    ///     .count(5)
    ///     .duration(120)
    ///     .build()?;
    /// let response = vouchers_api.create(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn vouchers(&self) -> VoucherApi {
        VoucherApi::new(self)
    }
}

// Implement Clone for UnifiClient
impl Clone for UnifiClient {
    fn clone(&self) -> Self {
        UnifiClient {
            config: self.config.clone(),
            http_client: self.http_client.clone(),
            auth_state: self.auth_state.clone(),
        }
    }
}
