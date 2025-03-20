use serde::{Deserialize, Serialize};

use crate::UnifiError;

/// Request to authorize a guest.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorizeGuestRequest {
    /// Command to authorize a guest.
    pub cmd: String,
    /// Client MAC address.
    pub mac: String,
    /// Minutes until authorization expires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    /// Upload speed limit in Kbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<u32>,
    /// Download speed limit in Kbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<u32>,
    /// Data transfer limit in MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u32>,
    /// AP MAC address to which client is connected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ap_mac: Option<String>,
}

impl TryFrom<GuestConfig> for AuthorizeGuestRequest {
    type Error = UnifiError;

    fn try_from(config: GuestConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            cmd: "authorize-guest".to_string(),
            mac: config.mac,
            minutes: config.minutes,
            up: config.up,
            down: config.down,
            bytes: config.transfer_limit,
            ap_mac: config.ap_mac,
        })
    }
}

/// Configuration for authorizing guests.
#[derive(Default)]
pub struct GuestConfig {
    /// Client MAC address
    pub mac: String,
    /// Minutes until authorization expires
    pub minutes: Option<u32>,
    /// Upload speed limit in Kbps
    pub up: Option<u32>,
    /// Download speed limit in Kbps
    pub down: Option<u32>,
    /// Data transfer limit in MB
    pub transfer_limit: Option<u32>,
    /// AP MAC address to which client is connected
    pub ap_mac: Option<String>,
}

impl GuestConfig {
    /// Create a new guest configuration builder.
    pub fn builder() -> GuestConfigBuilder {
        GuestConfigBuilder::default()
    }
}

/// Builder for guest configuration.
#[derive(Default)]
pub struct GuestConfigBuilder {
    config: GuestConfig,
}

impl GuestConfigBuilder {
    /// Set the client MAC address.
    pub fn mac(mut self, mac: impl Into<String>) -> Self {
        self.config.mac = mac.into().to_lowercase();
        self
    }

    /// Set the duration in minutes until authorization expires.
    pub fn minutes(mut self, minutes: u32) -> Self {
        self.config.minutes = Some(minutes);
        self
    }

    /// Set the upload speed limit in Kbps.
    pub fn upload_limit(mut self, up: u32) -> Self {
        self.config.up = Some(up);
        self
    }

    /// Set the download speed limit in Kbps.
    pub fn download_limit(mut self, down: u32) -> Self {
        self.config.down = Some(down);
        self
    }

    /// Set the data transfer limit in MB.
    pub fn transfer_limit(mut self, limit: u32) -> Self {
        self.config.transfer_limit = Some(limit);
        self
    }

    /// Set the AP MAC address to which the client is connected.
    pub fn ap_mac(mut self, ap_mac: impl Into<String>) -> Self {
        self.config.ap_mac = Some(ap_mac.into().to_lowercase());
        self
    }

    /// Build the guest configuration.
    pub fn build(self) -> Result<GuestConfig, UnifiError> {
        if self.config.mac.is_empty() {
            return Err(UnifiError::ApiError("MAC address is required".to_string()));
        }
        Ok(self.config)
    }
}

/// Represents the status and details of a guest authorization
///
/// Fields used by all guest entries:
/// - `id`: The unique identifier for this authorization
/// - `authorized_by`: Who or what authorized the guest
/// - `end`: When the authorization ends (Unix timestamp)
/// - `expired`: Whether the authorization has expired
/// - `mac`: The MAC address of the authorized guest
/// - `site_id`: The site ID where this guest was authorized
/// - `start`: When the authorization starts (Unix timestamp)
///
/// Fields used by active guest entries:
/// - `bytes`: The total data transfer limit in MB
/// - `rx_bytes`: The total data received in MB
/// - `tx_bytes`: The total data transmitted in MB
///
/// Fields used by expired guest entries:
/// - `unauthorized_by`: Who or what unauthorized the guest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GuestEntry {
    /// A guest authorization that has been authorized but not yet connected to
    /// the network
    Inactive {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: u64,
        expired: bool,
        mac: String,
        site_id: String,
        start: u64,
    },
    /// An active guest authorization that hasn't expired
    Active {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: u64,
        expired: bool, // Will be false
        mac: String,
        site_id: String,
        start: u64,
        // Active guests always have these fields
        bytes: u64,
        rx_bytes: u64,
        tx_bytes: u64,
    },
    /// An expired guest authorization
    Expired {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: u64,
        expired: bool, // Will be true
        mac: String,
        site_id: String,
        start: u64,
        // Optional field indicating if guest was explicitly unauthorized
        unauthorized_by: Option<String>,
    },
}

impl GuestEntry {
    /// Returns true if the guest was explicitly unauthorized rather than just
    /// expired
    pub fn was_unauthorized(&self) -> bool {
        match self {
            GuestEntry::Active { .. } => false,
            GuestEntry::Expired {
                unauthorized_by, ..
            } => unauthorized_by.is_some(),
            GuestEntry::Inactive { .. } => false,
        }
    }
}

/// Request to unauthorize a guest.
#[derive(Debug, Clone, Serialize)]
pub struct UnauthorizeGuestRequest {
    /// Command to unauthorize a guest.
    pub cmd: String,
    /// Client MAC address.
    pub mac: String,
}

impl UnauthorizeGuestRequest {
    /// Create a new guest unauthorization request.
    pub fn new(mac: impl Into<String>) -> Self {
        Self {
            cmd: "unauthorize-guest".to_string(),
            mac: mac.into().to_lowercase(),
        }
    }
}
