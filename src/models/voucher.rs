use serde::{Deserialize, Serialize};
use std::fmt;

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

/// Response from creating a voucher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVoucherResponse {
    /// When the voucher was created (Unix timestamp).
    pub create_time: u64,
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

// impl CreateVoucherRequest {
//     /// Create a new voucher creation request.
//     pub fn new(count: u32, minutes: u32) -> Self {
//         Self {
//             cmd: "create-voucher".to_string(),
//             n: count,
//             minutes,
//             note: None,
//             up: None,
//             down: None,
//             bytes: None,
//         }
//     }

//     /// Add a note to the voucher.
//     pub fn with_note(mut self, note: impl Into<String>) -> Self {
//         self.note = Some(note.into());
//         self
//     }

//     /// Set the upload speed limit.
//     pub fn with_upload_limit(mut self, kbps: u32) -> Self {
//         self.up = Some(kbps);
//         self
//     }

//     /// Set the download speed limit.
//     pub fn with_download_limit(mut self, kbps: u32) -> Self {
//         self.down = Some(kbps);
//         self
//     }

//     /// Set the data quota (in MB).
//     pub fn with_data_quota_mb(mut self, mb: u32) -> Self {
//         self.bytes = Some(mb as u64 * 1024 * 1024);
//         self
//     }
// }

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
