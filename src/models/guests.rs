use serde::{Deserialize, Serialize};

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
    pub ap_mac: String,
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
    /// use unifi_client::models::guests::UnauthorizeGuestRequest;
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
