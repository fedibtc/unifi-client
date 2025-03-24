use serde_json::json;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json, header};

mod common;

use common::setup_test_client;
use unifi_client::models::guest::GuestEntry;
use unifi_client::UniFiError;

#[tokio::test]
async fn test_authorize_guest() {
    let mock_server = MockServer::start().await;

    let duration = 30;
    
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
    
    // Set up guest authorize mock
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/stamgr"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({
            "cmd": "authorize-guest",
            "mac": "00:11:22:33:44:55",
            "minutes": duration
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [{
                    "_id": "guest1",
                    "mac": "00:11:22:33:44:55",
                    "authorized_by": "api",
                    "start": 1622548800,
                    "end": 1622550600,
                    "expired": false,
                    "site_id": "default"
                }]
            })))
        .mount(&mock_server)
        .await;

    let unifi_client = setup_test_client(&mock_server.uri()).await;
    
   let guest = unifi_client.guests()
      .authorize("00:11:22:33:44:55")
      .duration(duration)
      .send()
      .await
      .unwrap();
    
    match guest {
        GuestEntry::Inactive { expired, mac, start, end,.. } => {
            assert_eq!(mac, "00:11:22:33:44:55");
            assert!(!expired);
            assert!(end - start == duration as i64 * 60);
        },
        _ => panic!("Expected Inactive guest entry"),
    }
}

#[tokio::test]
async fn test_list_guests() {
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up guest list mock
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/guest"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [
                    {
                        "_id": "guest1",
                        "mac": "00:11:22:33:44:55",
                        "authorized_by": "api",
                        "start": 1622548800,
                        "end": 1622550600,
                        "expired": false,
                        "site_id": "default"
                    },
                    {
                        "_id": "guest2",
                        "mac": "aa:bb:cc:dd:ee:ff",
                        "authorized_by": "api",
                        "start": 1622548800,
                        "end": 1622550600,
                        "expired": true,
                        "site_id": "default",
                        "unauthorized_by": "api"
                    }
                ]
            })))
        .mount(&mock_server)
        .await;

    let unifi_client = setup_test_client(&mock_server.uri()).await;
    
    let guests = unifi_client.guests().list().send().await.unwrap();
    
    assert_eq!(guests.len(), 2);
    
    match &guests[0] {
        GuestEntry::Inactive { mac, expired, .. } => {
            assert_eq!(mac, "00:11:22:33:44:55");
            assert!(!expired);
        },
        _ => panic!("Expected Inactive guest entry"),
    }
    
    match &guests[1] {
        GuestEntry::Inactive { mac, expired, unauthorized_by, .. } => {
            assert_eq!(mac, "aa:bb:cc:dd:ee:ff");
            assert!(expired);
            assert_eq!(unauthorized_by.as_deref(), Some("api"));
        },
        _ => panic!("Expected Inactive guest entry"),
    }
}

#[tokio::test]
async fn test_list_guests_with_within() {
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up guest list mock with within parameter
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/guest"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({"within": 24})))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
        .mount(&mock_server)
        .await;

    let client = setup_test_client(&mock_server.uri()).await;
    
    // Use the builder and .send(), including .within()
    let guests = client.guests().list().within(24).send().await.unwrap();
    assert!(guests.is_empty());
}

#[tokio::test]
async fn test_unauthorize_guest() {
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up guest unauthorize mock
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/stamgr"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({
            "cmd": "unauthorize-guest",
            "mac": "00:11:22:33:44:55"
        })))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
        .mount(&mock_server)
        .await;

    let client = setup_test_client(&mock_server.uri()).await;

    // Use the builder and .send()
    let result = client.guests().unauthorize("00:11:22:33:44:55").send().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unauthorize_all_guests() {
    let mock_server = MockServer::start().await;

    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Mock for listing guests
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/guest"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": [
                    {
                        "_id": "guest1",
                        "mac": "00:11:22:33:44:55",
                        "authorized_by": "api",
                        "start": 1622548800,
                        "end": 1622550600,
                        "expired": false,
                        "site_id": "default"
                    },
                    {
                        "_id": "guest2",
                        "mac": "aa:bb:cc:dd:ee:ff",
                        "authorized_by": "api",
                        "start": 1622548800,
                        "end": 1622550600,
                        "expired": true,
                        "site_id": "default",
                        "unauthorized_by": "api"
                    }
                ]
            })))
        .mount(&mock_server)
        .await;

    // Mock for unauthorizing guests (expect TWO calls)
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/stamgr"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({"cmd": "unauthorize-guest", "mac": "00:11:22:33:44:55"})))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({"meta": { "rc": "ok" }, "data": []})))
        .mount(&mock_server)
        .await;

     Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/stamgr"))
        .and(header("cookie", "unifises=test-cookie"))
        .and(body_json(json!({"cmd": "unauthorize-guest", "mac": "aa:bb:cc:dd:ee:ff"})))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({"meta": { "rc": "ok" }, "data": []})))
        .mount(&mock_server)
        .await;

    let client = setup_test_client(&mock_server.uri()).await;

    let result = client.guests().unauthorize_all().send().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_guest_api_error() {
    let mock_server = MockServer::start().await;
    
    // Set up authentication mock
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            }))
            .insert_header("set-cookie", "unifises=test-cookie"))
        .mount(&mock_server)
        .await;
    
    // Set up guest list mock with error
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/guest"))
        .and(header("cookie", "unifises=test-cookie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "meta": { 
                    "rc": "error",
                    "msg": "Invalid guest parameters"
                },
                "data": []
            })))
        .mount(&mock_server)
        .await;

    let client = setup_test_client(&mock_server.uri()).await;
    
    let result = client.guests().list().send().await;
    
    assert!(result.is_err());
    match result {
        Err(UniFiError::ApiError(msg)) => {
            assert_eq!(msg, "Invalid guest parameters");
        },
        _ => panic!("Expected ApiError"),
    }
}