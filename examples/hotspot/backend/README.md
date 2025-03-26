# UniFi Cafe Backend

This is the backend service for the UniFi Cafe product – an MVP that enables guests to purchase
internet access for a specified duration. The backend authenticates with a UniFi Controller,
authorizes guests via a dedicated API endpoint, and serves a React Router Single Page Application
(SPA) as static files (the frontend).

## Features

- **Guest Authorization**: Exposes a POST endpoint (`/guests/authorize`) for authorizing guest
  network access.
- **UniFi Controller Integration**: Logs into and interacts with a UniFi controller.
- **Static Frontend Serving**: Serves the React SPA from a configurable directory.
- **CORS Support**: Configured to allow integration with external frontend clients.
- **Validation**: Uses [validator](https://docs.rs/validator) for request payload validation.
- **Logging**: Integrated logging via `tracing` and `tracing-subscriber`.

## Configuration

The backend uses environment variables for configuration. Copy the provided `.env.example` to `.env`
and modify your settings as needed.

## Running

### Development

To run the backend in development mode:

```bash
cargo run
```

This starts the server on the port specified in your `.env` (default is `8080`).

### Production Build

For production, build an optimized binary:

```bash
cargo build --release
```

The release binary will be located in `target/release/unifi-cafe`.

## API Endpoints

### Guest Authorization

- **Endpoint**: `POST /guests/authorize`
- **Description**: Authorizes a guest for internet access based on parameters such as client MAC
  address, duration, data quota, and additional connection metadata.

**Request Body Example:**
```json
{
  "client_mac_address": "00:11:22:33:44:55",
  "duration_minutes": 60,
  "data_quota_megabytes": 512,
  "access_point_mac_address": "aa:bb:cc:dd:ee:ff",
  "captive_portal_timestamp": 1735689600,
  "requested_url": "https://example.com",
  "wifi_network": "MyHotspot"
}
```

**Response Example:**
```json
{
  "expires_at": 1742673091,
  "guest_id": "67ddb99d01f8891b33c1ad97"
}
```

- `expires_at`: Unix timestamp (in seconds) when the authorization expires.
- `guest_id`: Unique identifier assigned by the UniFi controller for this guest authorization.

### Health Check

```
GET /health
```

Returns "OK" if the service is running.

## Static Frontend Serving

The backend uses the `FRONTEND_DIR` environment variable to serve the React SPA. The directory
should contain your built frontend assets (e.g. the output of `pnpm run build` for your React app).

## Deployment Instructions

To run the Rust backend on an Ubuntu server so that it starts automatically and restarts on failure,
use a systemd service.

1. Create a systemd service file:
    Create a file called `/etc/systemd/system/myapp.service` (adjust paths and username as needed):
    ```ini
    [Unit]
    Description=Rust Backend Service for React SPA
    After=network.target

    [Service]
    Type=simple
    WorkingDirectory=/path/to/your/app
    ExecStart=/path/to/your/app/unifi-cafe
    Restart=on-failure
    RestartSec=5
    User=yourusername
    Environment=RUST_LOG=info

    [Install]
    WantedBy=multi-user.target
    ```

2. Reload systemd and start the service:
    ```bash
    sudo systemctl daemon-reload
    sudo systemctl start myapp
    sudo systemctl enable myapp
    ```

This configuration ensures the service starts at boot and is automatically restarted if it fails.

## Additional Notes

- Ensure your `.env` file is correctly configured on the server.
- Update the `FRONTEND_DIR` variable as needed to point to the correct static assets.
- For further customization, consult the [axum] and [tower-http] documentation.

[axum]: https://docs.rs/axum/
[tower-http]: https://docs.rs/tower-http/