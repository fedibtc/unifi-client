# UniFi API Validator

This example demonstrates how to validate the UniFi API responses by testing various endpoints and their expected behaviors.

## Features

- Validates voucher creation and retrieval
- Validates site information retrieval
- Supports testing specific components or all components at once

## Usage

```bash
# Basic usage with all validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password

# Run only voucher validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password voucher

# Run only site validations
cargo run --example api_validator -- -c https://your-controller:8443 -u your-username -p your-password site
```

## Arguments

- `-c, --controller_url`: The UniFi controller URL (e.g., https://unifi.example.com:8443)
- `-u, --username`: Your UniFi controller username
- `-p, --password`: Your UniFi controller password
- `--command`: Optional subcommand to run specific validations:
  - `voucher`: Run only voucher validations
  - `site`: Run only site validations
  - `all`: Run all validations (default)

## Validations

### Voucher Validations
- Tests voucher creation with simple duration (30 minutes)
- Tests voucher creation with hour-based duration (5 hours)
- Validates voucher retrieval and duration values

### Site Validations
- Tests site information retrieval
- Validates site name field presence

## Example Output


```bash
# Run voucher validator
cargo run --example api_validator -- -c https://your-controller:8443 -n your-username -p your-password voucher

Running voucher validator...
Testing voucher with simple duration...
✅ Simple 'expire' duration test passed (30 minutes)
Testing voucher with minute unit...
✅ Minute unit duration test passed (5 minutes = 300 seconds)
Testing voucher with hour unit...
✅ Hour unit duration test passed (5 hours = 18000 seconds)
Testing voucher note...
✅ Voucher note test passed
```