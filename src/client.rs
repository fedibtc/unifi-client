use reqwest::{
    Client as ReqwestClient, Method, StatusCode,
    header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue},
};
use rpassword::prompt_password;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fmt;
use std::time::{Duration, Instant};
use url::Url;

use crate::{
    ApiResponse, EmptyResponse, LoginRequest, SiteApi, UnifiError, UnifiResult, VoucherApi,
};

/// Configuration for the UniFi client.
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
    /// Create a new client configuration builder.
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::default()
    }
}

/// Builder for client configuration.
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
    /// Set the controller URL.
    pub fn controller_url(mut self, url: &str) -> Self {
        self.controller_url = Some(url.to_string());
        self
    }

    /// Set the username for authentication.
    pub fn username(mut self, username: &str) -> Self {
        self.username = Some(username.to_string());
        self
    }

    /// Set the password for authentication.
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    /// Set the site to use.
    pub fn site(mut self, site: &str) -> Self {
        self.site = Some(site.to_string());
        self
    }

    /// Set whether to verify SSL certificates.
    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.verify_ssl = verify;
        self
    }

    /// Set the HTTP request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set a custom user agent string.
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// Build the client configuration.
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
    last_auth_time: Instant,
}

/// The main UniFi client for interacting with the UniFi Controller API.
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
    /// Create a new UniFi client with the given configuration.
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

    /// Login to the UniFi controller.
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
            last_auth_time: Instant::now(),
        });

        Ok(())
    }

    /// Ensure the client is authenticated.
    async fn ensure_authenticated(&mut self) -> UnifiResult<()> {
        if self.auth_state.is_none() {
            return Err(UnifiError::NotAuthenticated);
        }

        // Check if authentication is older than 1 hour
        let auth_age = self.auth_state.as_ref().unwrap().last_auth_time.elapsed();
        if auth_age > Duration::from_secs(3600) {
            // Re-authenticate
            self.login(None).await?;
        }

        Ok(())
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

    /// Get the voucher API.
    pub fn vouchers(&self) -> VoucherApi {
        VoucherApi::new(self)
    }

    /// Get the site API.
    pub fn sites(&self) -> SiteApi {
        SiteApi::new(self)
    }

    /// Get the current site ID.
    pub fn site(&self) -> &str {
        &self.config.site
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
