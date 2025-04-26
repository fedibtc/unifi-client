# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/fedibtc/unifi-client/releases/tag/v0.3.1) - 2025-04-26

### Added

- Hotspot Frontend MVP
- Improve security by protecting passwords stored in memory
- Add New to GuestEntry and convenience methods
- Add unauthorize_all() to Guest API
- Add mac() convenience function to GuestEntry
- Added support and examples for guest authorization API
- Add voucher models and get_by_create_time() method
- Add raw_request() to UnifiClient
- Initial implementation

### Fixed

- Update React Router dependencies to patch CVE
- Remove blocking lock in Debug for UniFiClient
- Resolve bug in UniFiClient singleton handling
- Re-implement UnifiClient ensure_authenticated() to resolve bug
- Correct data_quota to use u64 data type
- Simplified GuestEntry enum to Active and Inactive
- Add RUST_SRC_PATH to Nix flake

### Other

- Grant release-plz-release job permission to read PRs
- Run on main or master
- Add release-plz GitHub Actions workflow
- Add CI workflow ([#6](https://github.com/fedibtc/unifi-client/pull/6))
- *(deps)* Bump the cargo group across 2 directories with 1 update
- Bump unifi-client version to 0.3.0
- Use Clone derivation for UniFiClient rather than manual definition
- Improve README to explain two patterns for client creation and access
- Replace Mutex with RwLock for auth_state
- Use SecretString for csrf token and mark headers as sensitive
- Change ordering and grouping of UniFiClient in preparation for HTTP Method refactor
- Change rustfmt comment max line length to 100
- Rename API and models to guests and sites
- Remove no longer used GuestConfig and GuestConfigBuilder
- Rename variables to be more intuitively descriptive
- Update README
- Major refactor of the UniFiClient to simplify design and remove voucher support
- Rename all Unifi* to UniFi*
- Set rustfmt line length to 100 chars
- Replace duplicative code with get_auth_headers helper
- Apply rustfmt
- Improve documentation and style consistency of API implementations
- Improve documentation and style consistency of UniFi Client
- Improve documentation and style consistency of Site API and model
- Improve documentation and style consistency of Guest API and model
- Improve documentation and style consistency of Voucher API and model
- Change guest start and end data types to i64 to align with SystemTime and chrono
- Update dependencies
- Rename minutes to duration
- Rename transfer_limit to data_quota
- Minor improvements to docs and labels
- Add tests for Guest API
- Minor formatting / style changes
- Miscellaneous rustfmt changes
- Use builder pattern for creating vouchers
- Apply rustfmt changes
- Add clap dependency for CLI examples
- Add rustfmt config and change Nix flake to use latest nightly
- Update dependencies
- Alphabetize dependencies
- Add nix flake
- Hide crate, docs, and CI badge until published
- Initial commit
