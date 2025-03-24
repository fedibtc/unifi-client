# unifi-client

<!-- [![Crates.io](https://img.shields.io/crates/v/unifi-client.svg)](https://crates.io/crates/unifi-client) -->
<!-- [![Documentation](https://docs.rs/unifi-client/badge.svg)](https://docs.rs/unifi-client) -->
<!-- [![License](https://img.shields.io/crates/l/unifi-client.svg)](./LICENSE) -->
<!-- [![CI](https://github.com/fedibtc/unifi-client/workflows/CI/badge.svg)](https://github.com/fedibtc/unifi-client/actions) -->

A Rust client library for the Ubiquiti UniFi Controller API. This crate provides
a type-safe, async interface for interacting with UniFi controllers, allowing you
to manage guests, sites, and more.

> **Note:** This crate is not officially associated with or endorsed by Ubiquiti
> Inc.

## Features

## Features

- 🔐 Secure authentication with UniFi controllers
- 🌐 Access via a convenient singleton or multiple independent clients
- 🛜 Complete guest management: authorize, list, and unauthorize guests.
- 🔄 Async API with Tokio runtime support
- 🛡️ Comprehensive error handling
- 🧪 Well-tested (unit and integration tests)
- 🧩 Extensible architecture, ready for additional API endpoints

## Installation

Add `unifi-client` to your `Cargo.toml`:

```toml
[dependencies]
unifi-client = "0.1.0"  # Replace with the actual version
tokio = { version = "1", features = ["full"] }
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

## Creating Multiple Clients (Advanced)

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

## API Overview

### Guest Management

`unifi-client` uses a builder pattern for constructing API requests.

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
        .duration(60)
        .up(1024)
        .down(2048)
        .data_quota(1024)
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

This library has been tested with:
- UniFi Controller version 9.x

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
