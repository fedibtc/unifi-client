use serde::Deserialize;

/// Standard API response envelope from the UniFi controller.
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    /// Metadata about the response.
    pub meta: ApiMeta,
    
    /// The actual data returned, if any.
    pub data: Option<T>,
}

/// Metadata about an API response.
#[derive(Debug, Deserialize)]
pub struct ApiMeta {
    /// Result code. "ok" indicates success.
    pub rc: String,
    
    /// Error message, if any.
    pub msg: Option<String>,
}

/// Empty response type for endpoints that don't return meaningful data
#[derive(Debug, Deserialize)]
pub struct EmptyResponse {}