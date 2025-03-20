use serde::Serialize;

/// Request to login to the UniFi controller.
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    /// The username to authenticate with.
    pub username: String,

    /// The password to authenticate with.
    pub password: String,
}
