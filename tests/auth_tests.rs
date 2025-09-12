use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

use common::{
    add_auth_headers, api_path, setup_probe, setup_probe_and_login, setup_test_client,
    TestControllerKind,
};
use unifi_client::{UniFiClient, UniFiError};

#[tokio::test]
async fn test_successful_login() -> Result<(), UniFiError> {
    for &kind in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;

        setup_probe_and_login(&mock_server, kind).await;

        // Authenticated GET to /api/self or /proxy/network/api/self
        let endpoint = api_path(kind, "/api/self");
        let mock = Mock::given(method("GET")).and(path(endpoint.as_str()));
        let mock = add_auth_headers(mock, kind);
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": []
        })))
        .mount(&mock_server)
        .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let result = client.raw_request("GET", "/api/self", None::<()>).await;
        assert!(result.is_ok());
    }
    Ok(())
}

#[tokio::test]
async fn test_failed_login_invalid_credentials() -> Result<(), UniFiError> {
    for &kind in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe(&mock_server, kind).await;

        // Login failure status code differs by kind
        let status = match kind {
            TestControllerKind::Network => 400,
            TestControllerKind::Os => 403,
        };

        Mock::given(method("POST"))
            .and(path(kind.login_path()))
            .and(body_json(json!({
                "username": "test-user",
                "password": "wrong-password"
            })))
            .respond_with(ResponseTemplate::new(status))
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
        match (kind, unifi_client) {
            (TestControllerKind::Network, Err(UniFiError::AuthenticationError(msg))) => {
                assert_eq!(
                    msg,
                    "Authentication failed with status code: 400 Bad Request"
                );
            }
            (TestControllerKind::Os, Err(UniFiError::AuthenticationError(msg))) => {
                assert_eq!(msg, "Authentication failed with status code: 403 Forbidden");
            }
            _ => panic!("Expected AuthenticationError"),
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_login_server_error() {
    for &kind in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe(&mock_server, kind).await;

        Mock::given(method("POST"))
            .and(path(kind.login_path()))
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
}

#[tokio::test]
async fn test_login_no_cookies() {
    for &kind in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe(&mock_server, kind).await;

        // Network tends to return JSON body with rc ok; OS can return empty body with 200
        let resp = match kind {
            TestControllerKind::Network => ResponseTemplate::new(200).set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })),
            TestControllerKind::Os => ResponseTemplate::new(200),
        };

        Mock::given(method("POST"))
            .and(path(kind.login_path()))
            .respond_with(resp)
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
