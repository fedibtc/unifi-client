//! # unifi-client
//!
//! A Rust client library for the Ubiquiti UniFi Controller API.
//!
//! This crate provides a type-safe, async interface for interacting with UniFi
//! controllers, allowing you to manage vouchers, users, devices, and more.
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
//! use unifi_client::{ClientConfig, UniFiClient, VoucherConfig};
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
//!     let mut client = UniFiClient::new(config);
//!
//!     // Login - this will prompt for password if not provided
//!     client.login(None).await?;
//!
//!     // Create a voucher configuration
//!     let voucher_config =
//!         VoucherConfig::builder().count(5).duration(1440).note("Guest access").build()?;
//!
//!     // Create the vouchers
//!     let create_response = client.vouchers().create(voucher_config).await?;
//!
//!     // Print the voucher codes
//!     let vouchers = client.vouchers().get_by_create_time(create_response.create_time).await?;
//!     for voucher in vouchers {
//!         println!("Code: {}, Duration: {} minutes", voucher.code, voucher.duration);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod api;
mod client;
mod error;
mod models;

pub use api::guest::GuestApi;
pub use api::site::SiteApi;
pub use api::voucher::VoucherApi;
pub use client::{ClientConfig, UniFiClient};
pub use error::{UniFiError, UniFiResult};
pub use models::api_response::{ApiMeta, ApiResponse, EmptyResponse};
pub use models::auth::LoginRequest;
pub use models::guest::{AuthorizeGuestRequest, GuestConfig, GuestEntry, UnauthorizeGuestRequest};
pub use models::site::{Site, SiteStats};
pub use models::voucher::{
    CreateVoucherRequest, CreateVoucherResponse, Voucher, VoucherConfig, VoucherExpireUnit,
    VoucherStatus,
};
