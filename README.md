# unifi-client

<!-- [![Crates.io](https://img.shields.io/crates/v/unifi-client.svg)](https://crates.io/crates/unifi-client) -->
<!-- [![Documentation](https://docs.rs/unifi-client/badge.svg)](https://docs.rs/unifi-client) -->
<!-- [![License](https://img.shields.io/crates/l/unifi-client.svg)](./LICENSE) -->
<!-- [![CI](https://github.com/fedibtc/unifi-client/workflows/CI/badge.svg)](https://github.com/fedibtc/unifi-client/actions) -->

A Rust client library for the Ubiquiti UniFi Controller API. This crate provides
a type-safe, async interface for interacting with UniFi controllers.

> **Note:** This crate is not officially associated with or endorsed by Ubiquiti
  Inc.

## Features

- 🔐 Secure authentication with UniFi controllers
- 🎫 Complete voucher management (create, list, delete)
- 🔄 Async API with Tokio runtime support
- 🛡️ Comprehensive error handling
- 🧪 Well-tested with both unit and integration tests
- 📝 Fully documented API
- 🧩 Extensible architecture ready for additional API endpoints

## Installation

Add `unifi-client` to your `Cargo.toml`:

```toml
[dependencies]
unifi-client = "0.1.0"
```

## Quick Start

```rust
use unifi_client::{UnifiClient, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let config = ClientConfig::builder()
        .controller_url("https://unifi.example.com:8443")
        .username("admin")
        .site("default")
        .verify_ssl(false)
        .build()?;
    
    let mut client = UnifiClient::new(config);
    
    // Login - this will prompt for password if not provided
    client.login(None).await?;
    
    // Get list of sites
    println!("\nFetching available sites...");
    let sites = client.sites().list().await?;

    // Print site list
    println!("Available sites:");
    println!("{:<36} {:<20} {}", "ID", "Name", "Description");
    println!("{}", "-".repeat(80));
    
    for site in &sites {
        println!("{:<36} {:<20} {}", site.id, site.name, site.desc);
    }
    
    Ok(())
}
```

## Detailed Documentation

### Authentication

```rust
// With password prompt
client.login(None).await?;

// With explicit password
client.login(Some("my-password".to_string())).await?;
```

### Voucher Management

```rust
// Get the voucher API
let voucher_api = client.vouchers();

// Create vouchers
let new_vouchers = voucher_api.create(
    10,                           // count
    1440,                         // duration (minutes)
    Some("Conference".to_string()),  // note
    Some(10000),                  // up limit (Kbps)
    Some(20000),                  // down limit (Kbps)
    Some(1024),                   // data quota (MB)
).await?;

// List all vouchers
let all_vouchers = voucher_api.list().await?;

// Delete a specific voucher
voucher_api.delete(&voucher_id).await?;

// Delete all vouchers
voucher_api.delete_all().await?;
```

## Error Handling

The library uses a custom error type for all operations:

```rust
match client.login(None).await {
    Ok(_) => println!("Login successful!"),
    Err(UnifiError::AuthenticationError(msg)) => {
        eprintln!("Authentication failed: {}", msg);
    },
    Err(err) => eprintln!("Error: {}", err),
}
```

## Advanced Usage

### Custom HTTP Configuration

```rust
let config = ClientConfig::builder()
    .controller_url("https://unifi.example.com:8443")
    .username("admin")
    .site("default")
    .verify_ssl(false)
    .timeout(std::time::Duration::from_secs(30))
    .user_agent("My UniFi Client")
    .build()?;
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
