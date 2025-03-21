# UniFi Cafe Backend

This is an MVP backend for the UniFi Cafe Internet service, which allows guests
to purchase internet access for a specified duration.

## Features

- Authentication with UniFi Controller
- Guest authorization endpoint
- Session tracking
- CORS support for frontend integration

## Configuration

The backend uses environment variables for configuration. Create a `.env` file
with:

```
UNIFI_CONTROLLER_URL=https://your-unifi-controller:8443
UNIFI_USERNAME=your-username
UNIFI_PASSWORD=your-password
UNIFI_SITE=default
VERIFY_SSL=true
```

## Running

To run the backend:

```bash
cd examples/backend
cargo run
```

The server will start on port 3000 by default.

## API Endpoints

### Health Check

```
GET /health
```

Returns "OK" if the service is running.

### Guest Authorization

```
POST /guest/authorize
```

Authorizes a guest for internet access for 60 minutes and up to 512 megabytes of
data.

Request body:
```json
{
  "mac": "00:11:22:33:44:55",
  "data_quota_mb": 512,
  "duration_minutes": 60
}
```

Response:
```json
{
  "mac": "00:11:22:33:44:55",
  "data_quota": 512,
  "expires_at": 1742673091,
  "guest_id": "67ddb99d01f8891b33c1ad97"
}
```

- `mac`: The MAC address of the authorized device 
- `data_quota`: Optional data transfer limit in MB (null if no limit set)
- `expires_at`: Unix timestamp (in seconds) when the authorization expires
- `guest_id`: Unique identifier assigned by UniFi for this guest authorization
