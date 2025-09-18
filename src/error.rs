use thiserror::Error;
pub use url::ParseError as UrlParseError;

/// Error types for the UniFi API client.
#[derive(Error, Debug)]
pub enum UniFiError {
    /// Authentication failed with the UniFi controller.
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// The API returned an error.
    #[error("API error: {0}")]
    ApiError(String),

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Error parsing URL.
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] UrlParseError),

    /// The API endpoint/path string is invalid.
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),

    /// Error serializing or deserializing JSON.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Client is not authenticated.
    #[error("Not authenticated")]
    NotAuthenticated,

    /// Site not found.
    #[error("Site not found: {0}")]
    SiteNotFound(String),

    /// Invalid client configuration.
    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),
}

/// Result type for UniFi API operations.
pub type UniFiResult<T> = Result<T, UniFiError>;
