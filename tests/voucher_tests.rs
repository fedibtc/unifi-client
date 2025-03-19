use serde_json::json;
use unifi_client::{UnifiError, VoucherStatus};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json, header};

mod common;
use common::setup_test_client;

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
                        "status": "valid",
                        "qos_rate_max_down": 10000,
                        "qos_rate_max_up": 5000,
                        "bytes_quota": 1073741824
                    },
                    {
                        "_id": "voucher2",
                        "create_time": 1622548800,
                        "code": "DEF456",
                        "quota": 1,
                        "duration": 1440,
                        "used": 1,
                        "status": "used",
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
    assert_eq!(voucher1.rate_max_down, Some(10000));
    assert_eq!(voucher1.rate_max_up, Some(5000));
    assert_eq!(voucher1.bytes_quota, Some(1073741824));
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
    
    // Set up voucher create mock
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/hotspot"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({
            "cmd": "create-voucher",
            "n": 5,
            "minutes": 1440,
            "note": "Test vouchers"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
        .mount(&mock_server)
        .await;
    
    // Set up voucher list mock (for retrieving the created vouchers)
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/voucher"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [
                    {
                        "_id": "newvoucher1",
                        "create_time": 1622548800,
                        "code": "NEW123",
                        "quota": 1,
                        "duration": 1440,
                        "used": 0,
                        "note": "Test vouchers",
                        "status": "valid"
                    },
                    {
                        "_id": "newvoucher2",
                        "create_time": 1622548800,
                        "code": "NEW456",
                        "quota": 1,
                        "duration": 1440,
                        "used": 0,
                        "note": "Test vouchers",
                        "status": "valid"
                    }
                ]
            })))
        .mount(&mock_server)
        .await;
    
    // Create test client
    let client = setup_test_client(&mock_server.uri()).await;
    
    // Test creating vouchers
    let vouchers = client.vouchers().create(
        5,
        1440,
        Some("Test vouchers".to_string()),
        None,
        None,
        None
    ).await.unwrap();
    
    // Verify response
    assert_eq!(vouchers.len(), 2);
    assert_eq!(vouchers[0].code, "NEW123");
    assert_eq!(vouchers[0].note, Some("Test vouchers".to_string()));
    assert_eq!(vouchers[1].code, "NEW456");
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
        Err(UnifiError::ApiError(msg)) => {
            assert_eq!(msg, "Invalid site");
        },
        _ => panic!("Expected ApiError"),
    }
}