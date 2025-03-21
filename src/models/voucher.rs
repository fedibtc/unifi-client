use std::fmt;

use serde::{Deserialize, Serialize};

use crate::UnifiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A voucher is a code that allows a guest to connect to the network for a
/// limited time or with a data quota.
pub struct Voucher {
    /// The unique identifier for this voucher.
    #[serde(rename = "_id")]
    pub id: String,

    /// Name of the admin who created the voucher.
    pub admin_name: Option<String>,

    /// The voucher code that guests will enter.
    pub code: String,

    /// When the voucher was created (Unix timestamp).
    pub create_time: u64,

    /// The duration of the voucher in minutes.
    pub duration: u32,

    /// Whether this voucher is for a hotspot.
    pub for_hotspot: Option<bool>,

    /// Optional note associated with this voucher.
    pub note: Option<String>,

    /// Whether QoS settings override default settings.
    pub qos_overwrite: Option<bool>,

    /// Maximum download speed in Kbps, if set.
    pub qos_rate_max_down: Option<u32>,

    /// Maximum upload speed in Kbps, if set.
    pub qos_rate_max_up: Option<u32>,

    /// Data transfer limit in megabytes, if set.
    pub qos_usage_quota: Option<u64>,

    /// The number of times this voucher can be used.
    /// Value '0' is for multi-use, '1' is for single-use, and
    /// 'n' is for multi-use n times
    pub quota: u32,

    /// The site ID where this voucher was created.
    pub site_id: Option<String>,

    /// The current status of the voucher.
    pub status: VoucherStatus,

    /// When the voucher expires (Unix timestamp), 0 if not yet used.
    pub status_expires: Option<u64>,

    /// How many times this voucher has been used.
    pub used: u32,
}

impl fmt::Display for Voucher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code: {} ({})", self.code, self.status)
    }
}

/// Configuration for creating vouchers.
#[derive(Default)]
pub struct VoucherConfig {
    pub count: u32,
    pub duration: Option<u32>,
    pub note: Option<String>,
    pub up: Option<u32>,
    pub down: Option<u32>,
    pub data_quota: Option<u32>,
}

impl VoucherConfig {
    /// Create a new voucher configuration builder.
    pub fn builder() -> VoucherConfigBuilder {
        VoucherConfigBuilder::default()
    }
}

/// Builder for voucher configuration.
#[derive(Default)]
pub struct VoucherConfigBuilder {
    config: VoucherConfig,
}

impl VoucherConfigBuilder {
    /// Set the number of vouchers to create.
    pub fn count(mut self, count: u32) -> Self {
        self.config.count = count;
        self
    }

    /// Set the duration the voucher is valid after activation in minutes.
    pub fn duration(mut self, duration: u32) -> Self {
        self.config.duration = Some(duration);
        self
    }

    /// Set an optional note for the vouchers.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.config.note = Some(note.into());
        self
    }

    /// Set an optional upload speed limit in Kbps.
    pub fn upload_limit(mut self, up: u32) -> Self {
        self.config.up = Some(up);
        self
    }

    /// Set an optional download speed limit in Kbps.
    pub fn download_limit(mut self, down: u32) -> Self {
        self.config.down = Some(down);
        self
    }

    /// Set an optional data transfer quota in MB.
    pub fn data_quota(mut self, quota: u32) -> Self {
        self.config.data_quota = Some(quota);
        self
    }

    /// Build the voucher configuration.
    pub fn build(self) -> Result<VoucherConfig, UnifiError> {
        if self.config.count == 0 {
            return Err(UnifiError::ApiError(
                "Voucher count must be greater than 0".to_string(),
            ));
        }
        Ok(self.config)
    }
}

impl TryFrom<VoucherConfig> for CreateVoucherRequest {
    type Error = UnifiError;

    fn try_from(config: VoucherConfig) -> Result<Self, Self::Error> {
        if config.count == 0 {
            return Err(UnifiError::ApiError(
                "Voucher count must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            cmd: "create-voucher".to_string(),
            n: config.count,
            expire: config.duration,
            expire_number: None,
            expire_unit: None,
            quota: None,
            note: config.note,
            up: config.up,
            down: config.down,
            bytes: config.data_quota,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(into = "u32")]
pub enum VoucherExpireUnit {
    Seconds,
    Minutes,
    Hours,
}

impl From<VoucherExpireUnit> for u32 {
    fn from(unit: VoucherExpireUnit) -> u32 {
        match unit {
            VoucherExpireUnit::Seconds => 1,
            VoucherExpireUnit::Minutes => 60,
            VoucherExpireUnit::Hours => 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoucherStatus {
    /// The voucher is valid and has not been used.
    #[serde(rename = "VALID_ONE")]
    Valid,

    /// The voucher has been used.
    #[serde(rename = "USED")]
    Used,

    /// The voucher has expired.
    #[serde(rename = "EXPIRED")]
    Expired,
}

impl fmt::Display for VoucherStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoucherStatus::Valid => write!(f, "Valid"),
            VoucherStatus::Used => write!(f, "Used"),
            VoucherStatus::Expired => write!(f, "Expired"),
        }
    }
}

/// Request to create a voucher.
#[derive(Debug, Clone, Serialize)]
pub struct CreateVoucherRequest {
    /// Command to create a voucher.
    pub cmd: String,

    /// Number of vouchers to create.
    pub n: u32,

    /// Duration the voucher is valid after activation in minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<u32>,

    /// Duration the voucher is valid after activation per expire_unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_number: Option<u32>,

    /// Unit of time for the duration the voucher is valid after activation.
    /// Valid values are: 1 (minute), 60 (hour), 3600 (day)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_unit: Option<VoucherExpireUnit>,

    /// Optional single-use or multi-use.
    /// Value '0' is for multi-use, '1' is for single-use, and
    /// 'n' is for multi-use n times
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<u32>,

    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    /// Optional upload limit in Kbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<u32>,

    /// Optional download limit in Kbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<u32>,

    /// Optional data transfer limit in megabytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u32>,
}

/// Response from creating a voucher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVoucherResponse {
    /// When the voucher was created (Unix timestamp).
    pub create_time: u64,
}

// /// Request to delete a voucher.
// #[derive(Debug, Clone, Serialize)]
// pub struct DeleteVoucherRequest {
//     /// Command to delete a voucher.
//     pub cmd: String,

//     /// The ID of the voucher to delete.
//     #[serde(rename = "_id")]
//     pub id: String,
// }

// impl DeleteVoucherRequest {
//     /// Create a new voucher deletion request.
//     pub fn new(id: impl Into<String>) -> Self {
//         Self {
//             cmd: "delete-voucher".to_string(),
//             id: id.into(),
//         }
//     }
// }
