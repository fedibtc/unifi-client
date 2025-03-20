use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Serialize, Deserialize)]
/// A guest voucher.
pub struct Voucher {
    /// The unique identifier for this voucher.
    #[serde(rename = "_id")]
    pub id: String,
    
    /// When the voucher was created (Unix timestamp).
    pub create_time: u64,
    
    /// The voucher code that guests will enter.
    pub code: String,
    
    /// The number of times this voucher can be used.
    pub quota: u32,
    
    /// The duration of the voucher in minutes.
    pub duration: u32,
    
    /// How many times this voucher has been used.
    pub used: u32,
    
    /// Optional note associated with this voucher.
    pub note: Option<String>,
    
    /// The current status of the voucher.
    pub status: VoucherStatus,
    
    /// Maximum download speed in Kbps, if set.
    #[serde(rename = "qos_rate_max_down")]
    pub rate_max_down: Option<u32>,
    
    /// Maximum upload speed in Kbps, if set.
    #[serde(rename = "qos_rate_max_up")]
    pub rate_max_up: Option<u32>,
    
    /// Data quota in bytes, if set.
    pub bytes_quota: Option<u64>,
    
    /// When the voucher was last used (Unix timestamp).
    pub last_used_time: Option<u64>,
    
    /// The site ID where this voucher was created.
    pub site_id: Option<String>,
}

impl fmt::Display for Voucher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code: {} ({})", self.code, self.status)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VoucherStatus {
    /// The voucher is valid and has not been used.
    Valid,
    
    /// The voucher has been used.
    Used,
    
    /// The voucher has expired.
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

// /// Request to create a voucher.
// #[derive(Debug, Clone, Serialize)]
// pub struct CreateVoucherRequest {
//     /// Command to create a voucher.
//     pub cmd: String,
    
//     /// Number of vouchers to create.
//     pub n: u32,
    
//     /// Duration in minutes.
//     pub minutes: u32,
    
//     /// Optional note.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub note: Option<String>,
    
//     /// Optional upload limit in Kbps.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub up: Option<u32>,
    
//     /// Optional download limit in Kbps.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub down: Option<u32>,
    
//     /// Optional data quota in bytes.
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub bytes: Option<u64>,
// }

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
