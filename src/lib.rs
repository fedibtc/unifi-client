//! # unifi-client
//!
//! UniFiClient is a third party Ubiquiti UniFi API client, allowing you to
//! easily build your own UniFi integrations in Rust. UniFiClient comes with two
//! primary sets of APIs for communicating with UniFi controllers, a high level
//! strongly typed semantic API, and a lower level HTTP API for
//! extending behaviour.
//!
//! ## Adding UniFiClient
//!
//! Run this command in your terminal to add the latest version of UniFiClient.
//!
//! ```bash
//! cargo add unifi-client
//! ```
//!
//! ## Semantic API
//!
//! The semantic API is a high level API that provides a type-safe way to
//! interact with the UniFi controller. It is built around a set of [`models`]
//! that map to the UniFi controller's API. Currently the following modules are
//! available.
//!
//! - [`guest`] - Guest access management.
//!
//! ### Examples
//!
//! #### Authorizing a guest
//!
//! ```no_run
//! # use unifi_client::UniFiClient;
//! # #[tokio::main]
//! # async fn run() -> Result<(), unifi_client::UniFiError> {
//!     # let client = UniFiClient::builder()
//!     #    .controller_url("https://your-controller:8443")
//!     #    .username("your_username")
//!     #    .password_from_env("UNIFI_PASSWORD")
//!     #    .build()
//!     #    .await?;
//!     # unifi_client::initialize(client);
//! let unifi_client = unifi_client::instance();
//! // Authorize a guest with optional parameters.
//! let new_guest = unifi_client
//!     .guests()
//!     .authorize("00:11:22:33:44:55") // Required MAC address
//!     .duration_minutes(60) // Optional:  60 minutes
//!     .data_quota_megabytes(10) // Optional:  10 MB quota
//!     .send()
//!     .await?;
//!
//! println!("Authorized Guest with MAC: {}", new_guest.mac());
//! #   Ok(())
//! # }
//! ```
//!
//! All methods with multiple optional parameters are built as Builder structs,
//! allowing you to easily specify parameters.

mod api;
mod client;
mod error;

pub mod models;

pub use self::api::guest;
pub use self::client::{initialize, instance, UniFiClient, UniFiClientBuilder};
pub use self::error::{UniFiError, UniFiResult};
