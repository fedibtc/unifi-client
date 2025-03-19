use serde_json::json;
use unifi_client::{UnifiClient, ClientConfig, UnifiError};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json};

#[tokio::test]
async fn test_successful_login() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .and(body_json(json!({
            "username": "test-user",
            "password": "test-password"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Create test config
    let config = ClientConfig::builder()
        .controller_url(&mock_server.uri())
        .username("test-user")
        .password("test-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    // Create client
    let mut client = UnifiClient::new(config);
    
    // Test login
    let result = client.login(None).await;
    
    // Verify successful login
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_failed_login_invalid_credentials() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock for failure
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .and(body_json(json!({
            "username": "test-user",
            "password": "wrong-password"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { 
                    "rc": "error",
                    "msg": "Invalid username or password"
                },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie")
        )
        .mount(&mock_server)
        .await;
    
    // Create test config
    let config = ClientConfig::builder()
        .controller_url(&mock_server.uri())
        .username("test-user")
        .password("wrong-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    // Create client
    let mut client = UnifiClient::new(config);
    
    // Test login
    let result = client.login(None).await;
    
    // Verify failed login
    assert!(result.is_err());
    match result {
        Err(UnifiError::AuthenticationError(msg)) => {
            assert_eq!(msg, "Invalid username or password");
        },
        _ => panic!("Expected AuthenticationError"),
    }
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
    
    // Create test config
    let config = ClientConfig::builder()
        .controller_url(&mock_server.uri())
        .username("test-user")
        .password("test-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    // Create client
    let mut client = UnifiClient::new(config);
    
    // Test login
    let result = client.login(None).await;
    
    // Verify error
    assert!(result.is_err());
    match result {
        Err(UnifiError::AuthenticationError(msg)) => {
            assert!(msg.contains("500"));
        },
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
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{ "username": "test-user" }]
            }))
            // No set-cookie header
        )
        .mount(&mock_server)
        .await;
    
    // Create test config
    let config = ClientConfig::builder()
        .controller_url(&mock_server.uri())
        .username("test-user")
        .password("test-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    // Create client
    let mut client = UnifiClient::new(config);
    
    // Test login
    let result = client.login(None).await;
    
    // Verify error
    assert!(result.is_err());
    match result {
        Err(UnifiError::AuthenticationError(msg)) => {
            assert_eq!(msg, "No cookies received from server");
        },
        _ => panic!("Expected AuthenticationError"),
    }
}

#[tokio::test]
async fn test_explicit_password() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .and(body_json(json!({
            "username": "test-user",
            "password": "explicit-password"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{ "username": "test-user" }]
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Create test config
    let config = ClientConfig::builder()
        .controller_url(&mock_server.uri())
        .username("test-user")
        // No password in config
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    // Create client
    let mut client = UnifiClient::new(config);
    
    // Test login with explicit password
    let result = client.login(Some("explicit-password".to_string())).await;
    
    // Verify successful login
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_config_error() {
    // Test invalid URL
    let config_result = ClientConfig::builder()
        .controller_url("invalid-url")
        .username("test-user")
        .password("test-password")
        .site("default")
        .build();
    
    assert!(config_result.is_err());
    match config_result {
        Err(UnifiError::ConfigurationError(msg)) => {
            assert!(msg.contains("Invalid controller URL"));
        },
        _ => panic!("Expected ConfigurationError"),
    }
    
    // Test missing username
    let config_result = ClientConfig::builder()
        .controller_url("https://example.com")
        // No username
        .password("test-password")
        .site("default")
        .build();
    
    assert!(config_result.is_err());
    match config_result {
        Err(UnifiError::ConfigurationError(msg)) => {
            assert_eq!(msg, "Username is required");
        },
        _ => panic!("Expected ConfigurationError"),
    }
}