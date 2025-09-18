# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/fedibtc/unifi-client/compare/v0.3.2...v0.4.0) - 2025-09-18

### Added

- *(client,api,examples,tests)* [**breaking**] add request_json + get/post; change raw_request to return Response
- *(client)* add support for UniFi OS Server
- *(guests)* add optional QoS fields to GuestEntry variants

### Fixed

- *(security)* bump on-headers dep in hotspot example to resolve CVE-2025-7339
- *(security)* bump brace-expansion dep in hotspot example to resolve CVE-2025-5889
- *(client)* UniFi accepts and returns negative qos_usage_quota values

### Other

- *(examples)* bump react-router for hotspot example
- *(examples)* replace verify_ssl with accept_invalid_certs
- *(client)* harden auth state and request retry handling
- *(client)* Rename verify_ssl to accept_invalid_certs and make default false
- *(client)* improve runtime and builder-time config validation efficiency
- *(readme)* custom HTTP client example should include cookie store and disabled redirects
- *(client)* add request builder and simplify UniFiClient struct
- *(client)* add default-client feature flag and refactor global default instance
- *(client)* add test for negative qos_usage_quota in guests list
- *(client)* use http method enum in raw_request()
- *(client)* add send_http and route raw_request/request through it to deduplicate request logic
- deduplicate and refactor auth and guests tests
- minor test name change
- *(client)* rename list filter from within() to within_hours()
- *(client)* update login/auth tests for UniFi OS Server
- add .direnv to gitignore to speed up IDE file search
- *(client)* use reqwest cookie store; remove manual cookie header
- *(examples)* fix linting and example dependency issues
- *(deps)* Pin wiremock version to ensure rustc 1.74 MSRV
- *(readme)* update MSRV section to Rust 1.74
- *(cargo)* set rust-version to 1.74 (edition 2021)
- *(deps)* Update vite, ts, tailwind, and react in hotspot example frontend
- *(nix)* bump nixpkgs to 25.05 and update devShell toolchain

## [0.3.2](https://github.com/fedibtc/unifi-client/compare/v0.3.1...v0.3.2) - 2025-04-26

### Other

- Slim tokio dependency tree

## 0.3.1

### Added

- Hotspot Frontend MVP
- Serve frontend SPA with backend server

### Fixed

- Update `tokio` from 1.44.1 to 1.44.2 to patch CVE
- Update React Router dependencies to patch CVE

### Other

- Add release-plz GitHub Actions workflow
- Add CI workflow

## 0.3.0

### Added

- Improve security by protecting passwords stored in memory

### Fixed

- Remove blocking lock in Debug for UniFiClient
- Resolve bug in UniFiClient singleton handling

### Other

- Use Clone derivation for UniFiClient rather than manual definition
- Improve README to explain two patterns for client creation and access
- Replace Mutex with RwLock for auth_state
- Use SecretString for csrf token and mark headers as sensitive
- Change rustfmt comment max line length to 100
- Rename API and models to guests and sites
- Remove no longer used GuestConfig and GuestConfigBuilder

## 0.2.0

### Other

- Major refactor of the UniFiClient to simplify design and remove voucher support

## 0.1.0

### Added

- Add New to GuestEntry and convenience methods
- Add unauthorize_all() to Guest API
- Add mac() convenience function to GuestEntry
- Added support and examples for guest authorization API
- Add voucher models and get_by_create_time() method
- Add raw_request() to UnifiClient
- Initial implementation

### Other

- Add tests for Guest API
- Use builder pattern for creating vouchers
- Add clap dependency for CLI examples
- Add rustfmt config and change Nix flake to use latest nightly
- Add nix flake
