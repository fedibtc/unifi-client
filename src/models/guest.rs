use serde::{Deserialize, Serialize};

use crate::UniFiError;

/// Request to authorize a guest for network access.
///
/// This represents the API request body for the authorize-guest command.
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
    /// Data transfer quota in MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// AP MAC address to which client is connected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ap_mac: Option<String>,
}

/// Configuration for authorizing guest network access.
///
/// This struct is used to configure the parameters for guest authorization,
/// including duration, bandwidth limits, and data quotas.
#[derive(Default)]
pub struct GuestConfig {
    /// Client MAC address
    pub mac: String,
    /// Minutes until authorization expires
    pub duration: Option<u32>,
    /// Upload speed limit in Kbps
    pub up: Option<u32>,
    /// Download speed limit in Kbps
    pub down: Option<u32>,
    /// Data transfer quota in MB
    pub data_quota: Option<u64>,
    /// AP MAC address to which client is connected
    pub ap_mac: Option<String>,
}

impl GuestConfig {
    /// Create a new guest configuration builder.
    pub fn builder() -> GuestConfigBuilder {
        GuestConfigBuilder::default()
    }
}

/// Builder for creating guest configuration in a fluent style.
///
/// This builder provides a convenient way to create guest configurations
/// with optional parameters.
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
    pub fn duration(mut self, duration: u32) -> Self {
        self.config.duration = Some(duration);
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

    /// Set the data transfer quota in MB.
    pub fn data_quota(mut self, quota: u64) -> Self {
        self.config.data_quota = Some(quota);
        self
    }

    /// Set the AP MAC address to which the client is connected.
    pub fn ap_mac(mut self, ap_mac: impl Into<String>) -> Self {
        self.config.ap_mac = Some(ap_mac.into().to_lowercase());
        self
    }

    /// Build the guest configuration.
    pub fn build(self) -> Result<GuestConfig, UniFiError> {
        if self.config.mac.is_empty() {
            return Err(UniFiError::ApiError("MAC address is required".to_string()));
        }
        Ok(self.config)
    }
}

/// Represents the status and details of a guest network authorization.
///
/// This enum represents different states of a guest authorization in the UniFi
/// system. It can be in one of three states:
/// - Active: Guest is authorized and currently connected
/// - Inactive: Guest authorization exists but is not currently in use or has
///   expired
/// - New: Guest has just been authorized through the API
///
/// Fields used by all guest entries:
/// - `id`: The unique identifier for this authorization
/// - `authorized_by`: Who or what authorized the guest
/// - `end`: When the authorization ends (Unix timestamp)
/// - `mac`: The MAC address of the authorized guest
/// - `site_id`: The site ID where this guest was authorized
/// - `start`: When the authorization starts (Unix timestamp)
///
/// Fields used by Active guest entries:
/// - `bytes`: The total data transfer limit in MB
/// - `expired`: Whether the authorization has expired
/// - `rx_bytes`: The total data received in MB
/// - `tx_bytes`: The total data transmitted in MB
///
/// Fields used by Inactive guest entries:
/// - `expired`: Whether the authorization has expired
/// - `unauthorized_by`: Who or what unauthorized the guest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GuestEntry {
    /// An active guest authorization that hasn't expired
    Active {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: i64,
        expired: bool, // Will be false
        mac: String,
        site_id: String,
        start: i64,
        // Active guests always have these fields
        bytes: u64,
        rx_bytes: u64,
        tx_bytes: u64,
    },
    /// A guest authorization that has been authorized but not yet connected to
    /// the network or has expired
    Inactive {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: i64,
        expired: bool,
        mac: String,
        site_id: String,
        start: i64,
        // Optional field indicating if guest was explicitly unauthorized
        unauthorized_by: Option<String>,
    },
    /// A newly authorized guest (response from authorize-guest command)
    New {
        #[serde(rename = "_id")]
        id: String,
        authorized_by: String,
        end: i64,
        mac: String,
        site_id: String,
        start: i64,
    },
}

impl GuestEntry {
    /// Get who authorized the guest
    pub fn authorized_by(&self) -> &str {
        match self {
            GuestEntry::Active { authorized_by, .. } => authorized_by,
            GuestEntry::Inactive { authorized_by, .. } => authorized_by,
            GuestEntry::New { authorized_by, .. } => authorized_by,
        }
    }

    /// Get the expiration time of the guest authorization
    pub fn expires_at(&self) -> i64 {
        match self {
            GuestEntry::Active { end, .. } => *end,
            GuestEntry::Inactive { end, .. } => *end,
            GuestEntry::New { end, .. } => *end,
        }
    }

    /// Get the unique identifier for the guest
    pub fn id(&self) -> &str {
        match self {
            GuestEntry::Active { id, .. } => id,
            GuestEntry::Inactive { id, .. } => id,
            GuestEntry::New { id, .. } => id,
        }
    }

    /// Returns true if the guest authorization has expired
    pub fn is_expired(&self) -> bool {
        match self {
            GuestEntry::Active { expired, .. } => *expired,
            GuestEntry::Inactive { expired, .. } => *expired,
            GuestEntry::New { .. } => false,
        }
    }

    /// Get the MAC address of the guest
    pub fn mac(&self) -> &str {
        match self {
            GuestEntry::Active { mac, .. } => mac,
            GuestEntry::Inactive { mac, .. } => mac,
            GuestEntry::New { mac, .. } => mac,
        }
    }

    /// Returns true if the guest was explicitly unauthorized rather than just
    /// expired
    pub fn was_unauthorized(&self) -> bool {
        match self {
            GuestEntry::Active { .. } => false,
            GuestEntry::Inactive {
                unauthorized_by, ..
            } => unauthorized_by.is_some(),
            GuestEntry::New { .. } => false,
        }
    }
}

/// Request to revoke network access for a guest device.
///
/// This represents the API request body for the unauthorize-guest command.
#[derive(Debug, Clone, Serialize)]
pub struct UnauthorizeGuestRequest {
    /// Command to unauthorize a guest.
    pub cmd: String,
    /// Client MAC address.
    pub mac: String,
}

impl UnauthorizeGuestRequest {
    /// Creates a new request to unauthorize a guest device.
    ///
    /// # Arguments
    ///
    /// * `mac` - MAC address of the guest device to unauthorize
    ///
    /// # Examples
    ///
    /// ```
    /// use unifi_client::models::guest::UnauthorizeGuestRequest;
    ///
    /// let request = UnauthorizeGuestRequest::new("00:11:22:33:44:55");
    /// ```
    pub fn new(mac: impl Into<String>) -> Self {
        Self {
            cmd: "unauthorize-guest".to_string(),
            mac: mac.into().to_lowercase(),
        }
    }
}
