use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A UniFi site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    /// The unique identifier for this site.
    #[serde(rename = "_id")]
    pub id: String,

    /// The site name (used in API calls).
    pub name: String,

    /// The human-readable description of the site.
    pub desc: String,

    /// Role for this site (e.g., "admin").
    pub role: Option<String>,

    /// Whether this site is hidden.
    pub hidden: Option<bool>,

    /// Additional attributes for this site.
    #[serde(flatten)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Statistics for a site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteStats {
    /// Number of access points.
    pub num_ap: u32,

    /// Number of users.
    pub num_user: u32,

    /// Number of guests.
    pub num_guest: u32,

    /// Number of IoT devices.
    pub num_iot: Option<u32>,

    /// Status score.
    pub status: Option<String>,

    /// Health score.
    pub score: Option<f64>,

    /// Subsystem scores.
    pub subsystems: Option<Vec<SubsystemHealth>>,

    /// When the statistics were collected.
    pub timestamp: Option<u64>,

    /// Additional attributes.
    #[serde(flatten)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Health information for a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    /// The subsystem name.
    pub subsystem: String,

    /// Health score.
    pub score: f64,

    /// Status indicator.
    pub status: String,
}

impl std::fmt::Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.desc, self.name)
    }
}
