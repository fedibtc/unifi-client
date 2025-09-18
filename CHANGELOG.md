# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/fedibtc/unifi-client/compare/v0.3.2...v0.4.0) - 2025-09-18

### Added

- [**breaking**] Change raw_request() to return Response
- [**breaking**] Rename raw_request() to request()
- [**breaking**] Rename list filter from within() to within_hours()
- Add new convenience methods request_json, get, and post
- Add support for UniFi OS Server
- Add support for QoS fields in GuestEntry
- Use reqwest cookie store; remove manual cookie header management
- Add default-client feature flag and refactor global default instance

### Fixed

- Bump on-headers dep in hotspot example to resolve CVE-2025-7339
- Bump brace-expansion dep in hotspot example to resolve CVE-2025-5889

### Other

- Harden auth state and request retry handling
- Rename verify_ssl to accept_invalid_certs and make default false to improve security
- Improve runtime and builder-time config validation efficiency
- Updated docs and examples based on refactoring
- Deduplicate request logic by routing get(), post(), and request_json() through request()
- Improve and expand test coverage for auth, builder, and request
- Document and set MSRV to Rust 1.74
- Update vite, ts, tailwind, react, and react-router in hotspot example frontend
- Bump nixpkgs to 25.05 and update devShell toolchain

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
