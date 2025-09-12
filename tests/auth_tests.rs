use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

use common::setup_test_client;
use unifi_client::{UniFiClient, UniFiError};

#[cfg(test)]
mod unifi_network {
    use super::*;

    #[tokio::test]
    async fn test_successful_login() -> Result<(), UniFiError> {
        let mock_server = MockServer::start().await;

        // Set up authentication mock
        Mock::given(method("POST"))
            .and(path("/api/login"))
            .and(body_json(json!({
                "username": "test-user",
                "password": "test-password"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "meta": { "rc": "ok" },
                        "data": []
                    }))
                    .insert_header("set-cookie", "unifises=test-cookie"),
            )
            .mount(&mock_server)
            .await;

        let unifi_client = setup_test_client(&mock_server.uri()).await;

        // Test a different call, after the build/login
        // Set up a mock response for /api/self.
        Mock::given(method("GET"))
            .and(path("/api/self"))
            .and(header("cookie", "unifises=test-cookie"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
            .mount(&mock_server)
            .await;

        // Test an authenticated call
        let result = unifi_client
            .raw_request("GET", "/api/self", None::<()>)
            .await;

        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_failed_login_invalid_credentials() -> Result<(), UniFiError> {
        let mock_server = MockServer::start().await;

        // Set up authentication mock for failure (returning an error in the *body*)
        Mock::given(method("POST"))
            .and(path("/api/login"))
            .and(body_json(json!({
                "username": "test-user",
                "password": "wrong-password"
            })))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "meta": {
                    "rc": "error",
                    "msg": "api.err.Invalid"
                },
                "data": []
            })))
            .mount(&mock_server)
            .await;

        // Build the client, expect an error during the build() process (login failure).
        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("wrong-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        assert!(unifi_client.is_err()); // Expecting an error

        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert_eq!(
                    msg,
                    "Authentication failed with status code: 400 Bad Request"
                );
            }
            _ => panic!("Expected ApiError"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_login_server_error() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Set up authentication mock for server error
        Mock::given(method("POST"))
            .and(path("/api/login"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Build the client, expect an error during the build() process (login failure).
        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("test-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        // Verify error
        assert!(unifi_client.is_err());
        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert!(msg.contains("500"));
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[tokio::test]
    async fn test_login_no_cookies() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Set up authentication mock without cookies
        Mock::given(method("POST"))
            .and(path("/api/login"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "meta": { "rc": "ok" },
                    "data": [{ "username": "test-user" }]
                })), // No set-cookie header
            )
            .mount(&mock_server)
            .await;

        // Build the client, expect an error during the build() process (login failure).
        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("test-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        // Verify error
        assert!(unifi_client.is_err());
        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert_eq!(msg, "No cookies received from server");
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }
}

#[cfg(test)]
mod unifi_os {
    use chrono::{Duration, Utc};

    use super::*;

    #[tokio::test]
    async fn test_successful_login() -> Result<(), UniFiError> {
        // Construct test cookie for UniFi OS login
        let expires_duration = Utc::now() + Duration::hours(2);
        let expires_ms = expires_duration.timestamp_millis();
        let expires_http_date = expires_duration
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        const DUMMY_JWT: &str = "invalid.dummy.jwt";

        // Value used in Set-Cookie
        let set_cookie_value = format!(
            "TOKEN={}; path=/; expires={}; httponly; SameSite=None",
            DUMMY_JWT, expires_http_date
        );
        // Value that will appear in the request Cookie header
        let cookie_header = format!("TOKEN={}", DUMMY_JWT);

        let mock_server = MockServer::start().await;

        // Probe: OS returns 200 on HEAD "/"
        Mock::given(method("HEAD"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Set up authentication mock for UniFi OS at /api/auth/login
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .and(body_json(json!({
                "username": "test-user",
                "password": "test-password"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    // Cookie must be path=/ so it is sent to /proxy/network/*
                    .insert_header("set-cookie", set_cookie_value.clone())
                    // UniFi OS returns CSRF token headers
                    .insert_header("x-csrf-token", "test-csrf")
                    .insert_header("x-updated-csrf-token", "test-csrf")
                    // UniFi OS returns expires header
                    .insert_header("x-token-expire-time", expires_ms),
            )
            .mount(&mock_server)
            .await;

        let unifi_client = setup_test_client(&mock_server.uri()).await;

        // After login, authenticated call goes through proxy/network
        Mock::given(method("GET"))
            .and(path("/proxy/network/api/self"))
            .and(header("cookie", cookie_header))
            .and(header("x-csrf-token", "test-csrf"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let result = unifi_client
            .raw_request("GET", "/api/self", None::<()>)
            .await;

        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_failed_login_invalid_credentials() -> Result<(), UniFiError> {
        let mock_server = MockServer::start().await;

        // Probe as UniFi OS
        Mock::given(method("HEAD"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Set up authentication mock for failure at OS path
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .and(body_json(json!({
                "username": "test-user",
                "password": "wrong-password"
            })))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "code": "AUTHENTICATION_FAILED_INVALID_CREDENTIALS",
                "message": "Invalid username or password",
                "level": "debug"
            })))
            .mount(&mock_server)
            .await;

        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("wrong-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        assert!(unifi_client.is_err());
        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert_eq!(msg, "Authentication failed with status code: 403 Forbidden");
            }
            _ => panic!("Expected ApiError"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_login_server_error() {
        let mock_server = MockServer::start().await;

        // Probe as UniFi OS
        Mock::given(method("HEAD"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // OS login path, return 500
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("test-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        assert!(unifi_client.is_err());
        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert!(msg.contains("500"));
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[tokio::test]
    async fn test_login_no_cookies_unifi_os() {
        let mock_server = MockServer::start().await;

        // Probe as UniFi OS
        Mock::given(method("HEAD"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // OS login without cookies
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let unifi_client = UniFiClient::builder()
            .controller_url(mock_server.uri())
            .username("test-user")
            .password("test-password")
            .site("default")
            .verify_ssl(false)
            .build()
            .await;

        assert!(unifi_client.is_err());
        match unifi_client {
            Err(UniFiError::AuthenticationError(msg)) => {
                assert_eq!(msg, "No cookies received from server");
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }
}

#[tokio::test]
async fn test_config_error() {
    // Test invalid URL
    let unifi_client = UniFiClient::builder()
        .controller_url("invalid-url")
        .username("test-user")
        .password("test-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .await;

    assert!(unifi_client.is_err());
    match unifi_client {
        Err(UniFiError::ConfigurationError(msg)) => {
            assert!(msg.contains("Invalid controller URL"));
        }
        _ => panic!("Expected ConfigurationError"),
    }

    // Test missing username
    let unifi_client = UniFiClient::builder()
        .controller_url("https://example.com")
        // No username
        .password("test-password")
        .site("default")
        .build()
        .await;

    assert!(unifi_client.is_err());
    match unifi_client {
        Err(UniFiError::ConfigurationError(msg)) => {
            assert_eq!(msg, "Username is required");
        }
        _ => panic!("Expected ConfigurationError"),
    }
}
