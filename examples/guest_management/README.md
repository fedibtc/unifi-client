# UniFi Guest Management Example

This example demonstrates how to use the UniFi client library to manage guest
authorizations, including authorizing new guests, listing current guests, and
unauthorizing guests.

## Features

- List all guests with their details (MAC address, status, and who authorized them)
- Authorize new guests with custom duration
- Unauthorize individual guests
- Unauthorize all guests (with safety confirmation)
- Environment variable configuration support

## Usage

### Running with Environment Variables

```bash
# Set your UniFi controller details
export UNIFI_CONTROLLER="https://unifi.example.com:8443"
export UNIFI_USERNAME="admin"
export UNIFI_PASSWORD="password"
export UNIFI_SITE="default"
export UNIFI_VERIFY_SSL="false"

# Run the example
cargo run
```

### Running with Prompts

If environment variables are not set, the example will use defaults and prompt
for the password:

```bash
cargo run
```

## Environment Variables

- `UNIFI_CONTROLLER`: URL of your UniFi controller (default: "https://unifi.example.com:8443")
- `UNIFI_USERNAME`: Username for authentication (default: "admin")
- `UNIFI_PASSWORD`: Password for authentication (optional, will prompt if not set)
- `UNIFI_SITE`: Site to manage guests for (default: "default")
- `UNIFI_VERIFY_SSL`: Whether to verify SSL certificates (default: false)

## Menu Options

1. **List Guests**: Display all guests with their details
2. **Authorize Guest**: Authorize a new guest with:
   - MAC address
   - Duration (in minutes)
3. **Unauthorize Guest**: Select and unauthorize a specific guest
4. **Unauthorize All Guests**: Remove all guest authorizations from the system
5. **Exit**: Close the application

## Example Output

```bash
UNIFI_CONTROLLER="https://unifi.example.com:8443" UNIFI_USERNAME="admin" UNIFI_PASSWORD="password" UNIFI_SITE="default" cargo run

UniFi Guest Management Example
==============================
Controller: https://unifi.example.com:8443
Site: default
✅ Authentication successful!

Guest Management Options:
1. List Active Guests
2. List Expired Guests
3. Authorize Guest
4. Unauthorize Guest
5. Unauthorize All Guests
6. Exit

Select an option (1-6): 1

Fetching active guests...

Found 2 active guests:
ID         MAC                 Status       Expires At (UTC)
--------------------------------------------------------------------------------
00112233   00:11:22:33:44:55   Active       2025-09-12 17:35:14 UTC
aabbccdd   aa:bb:cc:dd:ee:ff   Expired      2025-09-11 18:46:21 UTC
```

## Notes

- The example includes safety confirmations for destructive operations
- Duration is specified in minutes (e.g., 1440 for 1 day)
- SSL verification is disabled by default for easier testing
- Guest MAC addresses must be in the format "00:11:22:33:44:55"
- Guests start in an Inactive state and become Active once they connect and transfer data
- Guests can be explicitly unauthorized or expire naturally based on their duration
