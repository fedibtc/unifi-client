# UniFi API Validator

This example demonstrates how to validate the UniFi API responses by testing various endpoints and
their expected behaviors.

## Features

- Command-driven validations for Guests and Sites
- Guest authorization flows: authorize, list, unauthorize
- Parameter handling: minutes range, MAC formats, data quota (bytes), speed limits (Kbps)
- Site endpoints: site info and list sites
- Clear pass/fail output with helpful context per check

## Usage

```bash
# Basic usage with all validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password

# Run only guest authorization validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password guests

# Run only site validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password sites
```

## Arguments

- `-c, --controller_url`: The UniFi controller URL (e.g., https://unifi.example.com:8443)
- `-u, --username`: Your UniFi controller username
- `-p, --password`: Your UniFi controller password
- `--command`: Optional subcommand to run specific validations:
  - `guests`: Run only guest authorization validations
  - `sites`: Run only site validations
  - `all`: Run all validations (default)

## Validations

### Guest Validations (`guests.rs`)
- Authorize with simple duration: verifies timestamps, exact duration, and core fields.
- List guests: ensures array response and validates required fields for each entry.
- Unauthorize guest: confirms request succeeds and returns an empty array.
- Minutes parameter range: exercises negative, zero, and large values; reports acceptance and any controller adjustments.
- MAC address formats: accepts standard, no-separator, hyphenated, uppercase, mixed-case; explores edge cases and normalization.
- Data quota (bytes): sets `qos_usage_quota`, checks `qos_overwrite` presence/boolean, validates exact byte quota; also tests alongside `minutes`.
- Speed limits (Kbps): validates `up` and `down` independently and together; asserts `qos_overwrite=true`; checks exact numeric matches for `qos_rate_max_up`/`qos_rate_max_down`; documents behavior for zero and negative values.

### Site Validations (`sites.rs`)
- Site info: verifies structure and `name` field presence.
- List sites: validates array response and reports empty vs. non-empty.
