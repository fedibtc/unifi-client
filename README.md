# unifi-client

[![Crates.io](https://img.shields.io/crates/v/unifi-client.svg)](https://crates.io/crates/unifi-client)
[![Documentation](https://docs.rs/unifi-client/badge.svg)](https://docs.rs/unifi-client)
[![License](https://img.shields.io/crates/l/unifi-client.svg)](./LICENSE)
[![CI](https://github.com/fedibtc/unifi-client/workflows/CI/badge.svg)](https://github.com/fedibtc/unifi-client/actions)

UniFiClient is a third party Ubiquiti UniFi API client, allowing you to easily build your own UniFi
integrations in Rust. UniFiClient comes with two primary sets of APIs for communicating with UniFi
controllers, a high level strongly typed semantic API, and a lower level HTTP API for extending
behaviour.

> **Note:** This crate is not officially associated with or endorsed by Ubiquiti
> Inc.

## Features

- 🔐 Secure authentication with UniFi controllers
- 🌐 Access via a convenient singleton or multiple independent clients
- 🛜 Complete guest management: authorize, list, and unauthorize guests.
- 🔄 Async API with Tokio runtime support
- 🛡️ Comprehensive error handling
- 🧪 Well-tested (unit and integration tests)
- 🧩 Extensible architecture, ready for additional API endpoints

## Installation

Run this command in your terminal to add the latest version of UniFiClient.

```bash
cargo add unifi-client
```

## Quick Start

The easiest way to use unifi-client is with the singleton pattern. This provides
a single, globally accessible client instance.

```rust
use unifi_client::{UniFiClient, UniFiError};

#[tokio::main]
async fn main() -> Result<(), UniFiError> {
    // 1. Initialize the singleton client (ONCE, at the start).
    //    Get your credentials securely (e.g., from environment variables).
    let client = UniFiClient::builder()
        .controller_url("https://your-unifi-controller:8443") // Replace!
        .username("your_username") // Replace!
        .password_from_env("UNIFI_PASSWORD") // Best practice
        .site("default")  // Or your site ID
        .verify_ssl(false)  // Set to `true` if you have valid SSL
        .build()
        .await?;
    unifi_client::initialize(client);

    // 2. Access the client anywhere using `unifi_client::instance()`:
    let guests = unifi_client::instance().guests().list().send().await?;
    println!("Guests: {:?}", guests);

    Ok(())
}
```

## API Overview

### Semantic API

The semantic API is a high level API that provides a type-safe way to
interact with the UniFi controller. It is built around a set of `models`
that map to the UniFi controller's API. Currently the following modules are
available.

- `guest` - Guest access management.

#### Authorizing a guest

`unifi-client` uses a builder pattern for constructing API requests.

All methods with multiple optional parameters are built as Builder structs, allowing you to easily
specify parameters.

```rust
use unifi_client::{UniFiClient, UniFiError};

#[tokio::main]
async fn main() -> Result<(), UniFiError> {
    let client = UniFiClient::builder()
        .controller_url("https://your-controller:8443")
        .username("your_username")
        .password_from_env("UNIFI_PASSWORD")
        .build()
        .await?;
    unifi_client::initialize(client);
    
    let unifi_client = unifi_client::instance();

    // Authorize a guest:
    let new_guest = unifi_client.guests()
        .authorize("00:11:22:33:44:55")
        .duration_minutes(60)
        .up(1024)
        .down(2048)
        .data_quota_megabytes(1024)
        .send() // *MUST* call .send()
        .await?;

    println!("Authorized Guest: {:?}", new_guest);

    // List guests (with optional filtering):
    let guests = unifi_client.guests().list().within(24).send().await?;

    // Unauthorize a guest:
    unifi_client.guests().unauthorize("00:11:22:33:44:55").send().await?;

    //Unathorize all guests
    unifi_client.guests().unauthorize_all().send().await?;
    Ok(())
}
```

## Error Handling

All API methods return a `Result<T, UniFiError>`.

```rust
use unifi_client::{UniFiClient, UniFiError};

#[tokio::main]
async fn main() -> Result<(), UniFiError> {
    let client = UniFiClient::builder()
        .controller_url("https://your-controller:8443")
        .username("your_username")
        .password_from_env("UNIFI_PASSWORD")
        .build()
        .await?;
    unifi_client::initialize(client);
    
    let result = unifi_client::instance().guests().list().send().await;
    
    match result {
       Ok(guests) => println!("Guests: {:?}", guests),
       Err(UniFiError::ApiError(msg)) => eprintln!("API Error: {}", msg),
       Err(UniFiError::AuthenticationError(msg)) => eprintln!("Auth Error: {}", msg),
       Err(e) => eprintln!("Other Error: {:?}", e),
    }
    Ok(())
}
```

## Advanced Usage

### Builder vs. Singleton: Choosing the Right Approach

UniFiClient supports two patterns for client creation and access:

- **Builder (`UniFiClient::builder()`):**  
  Create new, independent client instances with custom configurations.  
  **Use when:**
  - Connecting to multiple controllers in a single application.
  - Writing tests (e.g., with wiremock) or needing isolated configurations.
  - Spinning up short-lived clients for specific tasks.

- **Singleton (`initialize()` / `instance()`):**  
  Set up a global client for simplified, centralized access.  
  **Use when:**
  - Your app interacts with a single UniFi controller.
  - You want to avoid passing client instances around.
  - A uniform configuration is needed throughout your codebase.

**Best Practices:**
- **Single Controller:** Call `initialize()` early (typically in `main`) and use `instance()` anywhere else.
- **Multiple or Custom Instances:** Use the builder to create each client independently.
- If `initialize()` isn’t called, any singleton usage should gracefully return a configuration error.

### Creating Multiple Clients

If you need to connect to *multiple* UniFi Controllers, or you need different client configurations
within the same application, create independent client instances using `UniFiClient::builder()`
*without* calling `initialize()`:

```rust
use unifi_client::{UniFiClient, UniFiError};

#[tokio::main]
async fn main() -> Result<(), UniFiError> {
    let client1 = UniFiClient::builder()
        .controller_url("https://controller1.example.com:8443")
        .username("user1")
        .password("password1")
        .build()
        .await?;

    let client2 = UniFiClient::builder()
        .controller_url("https://controller2.example.com:8443")
        .username("user2")
        .password("password2")
        .build()
        .await?;

    // Use client1 and client2 independently.
    let guests1 = client1.guests().list().send().await?;
    let guests2 = client2.guests().list().send().await?;

    println!("Guests on controller 1: {:?}", guests1);
    println!("Guests on controller 2: {:?}", guests2);

   Ok(())
}
```

### Custom HTTP Client

```rust
use unifi_client::{UniFiClient, UniFiClientBuilder};
use reqwest::Client as ReqwestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let custom_http_client = ReqwestClient::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let client = UniFiClient::builder()
        .controller_url("https://your-controller:8443")
        .username("your_username")
        .password_from_env("UNIFI_PASSWORD")
        .http_client(custom_http_client)
        .build()
        .await?;
    Ok(())
}
```

## Planned Features

- [ ] Statistics and reporting

## Compatibility

### UniFi

This library has been tested with:
- UniFi Controller version 9.x

### Minimum Supported Rust Version (MSRV)

- MSRV: Rust 1.74.0 (edition 2021)

We set `rust-version = "1.74"` in `Cargo.toml`. Downstream crates can remain on older editions (e.g., 2018/2021); they just need a toolchain new enough to compile this crate.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the [MIT license](./LICENSE).

## Acknowledgements

- This library is inspired by various UniFi API clients in other languages
- Thanks to the Ubiquiti community for documenting the unofficial API
