//! # unifi-client
//!
//! A Rust client library for the Ubiquiti UniFi Controller API.
//!
//! This crate provides a type-safe, async interface for interacting with UniFi controllers,
//! allowing you to manage vouchers, users, devices, and more.
//!
//! ## Features
//!
//! - 🔐 Secure authentication with UniFi controllers
//! - 🎫 Complete voucher management (create, list, delete)
//! - 🔄 Async API with Tokio runtime support
//! - 🛡️ Comprehensive error handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use unifi_client::{UnifiClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client
//!     let config = ClientConfig::builder()
//!         .controller_url("https://unifi.example.com:8443")
//!         .username("admin")
//!         .site("default")
//!         .verify_ssl(false)
//!         .build()?;
//!     
//!     let mut client = UnifiClient::new(config);
//!     
//!     // Login - this will prompt for password if not provided
//!     client.login(None).await?;
//!     
//!     // Create vouchers
//!     let vouchers = client.vouchers().create(
//!         5,                           // count
//!         1440,                        // duration (minutes)
//!         Some("Event tickets".into()), // note
//!         None,                        // up limit
//!         None,                        // down limit
//!         None,                        // quota
//!     ).await?;
//!     
//!     // Print the voucher codes
//!     for voucher in vouchers {
//!         println!("Code: {}, Duration: {} minutes", voucher.code, voucher.duration);
//!     }
//!     
//!     Ok(())
//! }
//! ```

// Module declarations
mod api;
mod client;
mod error;
mod models;

// Public exports
pub use api::voucher::VoucherApi;
pub use api::site::SiteApi;
pub use client::{UnifiClient, ClientConfig};
pub use error::{UnifiError, UnifiResult};
pub use models::voucher::{Voucher, VoucherStatus};
pub use models::site::{Site, SiteStats};
pub use models::auth::LoginRequest;
pub use models::api_response::{ApiResponse, ApiMeta, EmptyResponse};