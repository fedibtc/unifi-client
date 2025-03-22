use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Represents a UniFi network site.
///
/// A site in UniFi represents a logical grouping of devices and settings.
/// Most UniFi controllers have at least one site, often named "default".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    /// The unique identifier for this site.
    #[serde(rename = "_id")]
    pub id: String,

    /// The site name used in API calls (e.g., "default").
    pub name: String,

    /// The human-readable description of the site (e.g., "Main Office").
    pub desc: String,

    /// The user's role for this site (e.g., "admin").
    pub role: Option<String>,

    /// Whether this site is hidden in the UI.
    pub hidden: Option<bool>,

    /// Additional attributes for this site.
    #[serde(flatten)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Statistics and health information for a UniFi site.
///
/// This represents the current operational status of a site,
/// including device counts and health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteStats {
   /// Number of access points connected to the site.
   pub num_ap: u32,

   /// Number of regular users/clients connected to the site.
   pub num_user: u32,

   /// Number of guest users connected to the site.
   pub num_guest: u32,

   /// Number of IoT devices connected to the site.
   pub num_iot: Option<u32>,

   /// Overall status of the site (e.g., "ok", "warning").
   pub status: Option<String>,

   /// Overall health score of the site (0-100).
   pub score: Option<f64>,

   /// Health information for individual subsystems.
   pub subsystems: Option<Vec<SubsystemHealth>>,

   /// When the statistics were collected (Unix timestamp).
   pub timestamp: Option<u64>,

   /// Additional attributes not explicitly defined.
   #[serde(flatten)]
   pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Health information for a specific network subsystem.
///
/// Represents the operational status of individual parts of the network
/// such as WAN, LAN, WLAN, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    /// The name of the subsystem (e.g., "wan", "lan", "wlan").
    pub subsystem: String,

    /// Health score for this subsystem (0-100).
    pub score: f64,

    /// Status indicator (e.g., "ok", "warning", "error").
    pub status: String,
}

impl std::fmt::Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.desc, self.name)
    }
}
