use serde_json::json;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json, header};

mod common;
use common::setup_test_client;
use unifi_client::{UniFiError, VoucherConfig, VoucherStatus};

#[tokio::test]
async fn test_list_vouchers() {
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
                "data": [{ "username": "test-user" }]
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up voucher list mock
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/voucher"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [
                    {
                        "_id": "voucher1",
                        "create_time": 1622548800,
                        "code": "ABC123",
                        "quota": 1,
                        "duration": 1440,
                        "used": 0,
                        "status": "VALID_ONE",
                        "qos_rate_max_down": 10000,
                        "qos_rate_max_up": 5000,
                        "qos_usage_quota": 1073741824
                    },
                    {
                        "_id": "voucher2",
                        "create_time": 1622548800,
                        "code": "DEF456",
                        "quota": 1,
                        "duration": 1440,
                        "used": 1,
                        "status": "USED",
                        "note": "Test voucher"
                    }
                ]
            })))
        .mount(&mock_server)
        .await;
    
    // Create test client
    let client = setup_test_client(&mock_server.uri()).await;
    
    // Test listing vouchers
    let vouchers = client.vouchers().list().await.unwrap();

    // Verify response
    assert_eq!(vouchers.len(), 2);
    
    // Check first voucher
    let voucher1 = &vouchers[0];
    assert_eq!(voucher1.id, "voucher1");
    assert_eq!(voucher1.code, "ABC123");
    assert_eq!(voucher1.status, VoucherStatus::Valid);
    assert_eq!(voucher1.qos_rate_max_down, Some(10000));
    assert_eq!(voucher1.qos_rate_max_up, Some(5000));
    assert_eq!(voucher1.qos_usage_quota, Some(1073741824));
    assert_eq!(voucher1.note, None);
    
    // Check second voucher
    let voucher2 = &vouchers[1];
    assert_eq!(voucher2.id, "voucher2");
    assert_eq!(voucher2.code, "DEF456");
    assert_eq!(voucher2.status, VoucherStatus::Used);
    assert_eq!(voucher2.note, Some("Test voucher".to_string()));
}

#[tokio::test]
async fn test_create_voucher() {
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
                "data": [{ "username": "test-user" }]
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/hotspot"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({
            "cmd": "create-voucher",
            "n": 5,
            "note": "Test vouchers",
            "expire": 1440,
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{
                    "create_time": 1622548800
                }]
            })))
        .mount(&mock_server)
        .await;

    // Create test client
    let client = setup_test_client(&mock_server.uri()).await;
    
    let voucher_config = VoucherConfig::builder()
        .count(5)
        .duration(1440)
        .note("Test vouchers")
        .build()
        .unwrap();
    
    // Test creating vouchers
    let voucher_response = client.vouchers().create(voucher_config).await.unwrap();
    
    // Verify response
    assert_eq!(voucher_response.create_time, 1622548800);
}

#[tokio::test]
async fn test_delete_voucher() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{ "username": "test-user" }]
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up voucher delete mock
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/hotspot"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({
            "cmd": "delete-voucher",
            "_id": "voucher1"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
        .mount(&mock_server)
        .await;
    
    // Create test client
    let client = setup_test_client(&mock_server.uri()).await;
    
    // Test deleting a voucher
    let result = client.vouchers().delete("voucher1").await;
    
    // Verify response
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_api_error() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{ "username": "test-user" }]
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up voucher list mock with error
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/voucher"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { 
                    "rc": "error",
                    "msg": "Invalid site"
                },
                "data": []
            })))
        .mount(&mock_server)
        .await;
    
    // Create test client
    let client = setup_test_client(&mock_server.uri()).await;
    
    // Test listing vouchers with API error
    let result = client.vouchers().list().await;
    
    // Verify error
    assert!(result.is_err());
    match result {
        Err(UniFiError::ApiError(msg)) => {
            assert_eq!(msg, "Invalid site");
        },
        _ => panic!("Expected ApiError"),
    }
}