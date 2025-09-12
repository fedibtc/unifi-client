use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

use common::{
    add_auth_headers, api_path, setup_probe_and_login, setup_test_client, TestControllerKind,
};
use unifi_client::models::guests::GuestEntry;

#[tokio::test]
async fn test_authorize_guest() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;

        setup_probe_and_login(&mock_server, flavor).await;

        let duration_minutes = 30;

        // Guest authorize endpoint
        let endpoint = api_path(flavor, "/api/s/default/cmd/stamgr");
        let mock = Mock::given(method("POST")).and(path(endpoint.as_str()));
        let mock = add_auth_headers(mock, flavor).and(body_json(json!({
            "cmd": "authorize-guest",
            "mac": "00:11:22:33:44:55",
            "minutes": duration_minutes,
            "ap_mac": "00:00:00:00:00:00",
        })));
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
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

        let client = setup_test_client(&mock_server.uri()).await;

        let guest = client
            .guests()
            .authorize("00:11:22:33:44:55")
            .duration_minutes(duration_minutes)
            .send()
            .await
            .unwrap();

        match guest {
            GuestEntry::Inactive {
                expired,
                mac,
                start,
                end,
                ..
            } => {
                assert_eq!(mac, "00:11:22:33:44:55");
                assert!(!expired);
                assert_eq!(end - start, duration_minutes as i64 * 60);
            }
            _ => panic!("Expected Inactive guest entry"),
        }
    }
}

#[tokio::test]
async fn test_list_guests() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe_and_login(&mock_server, flavor).await;

        let endpoint = api_path(flavor, "/api/s/default/stat/guest");
        let mock = Mock::given(method("GET")).and(path(endpoint.as_str()));
        let mock = add_auth_headers(mock, flavor);
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": [
                {
                    "authorized_by": "api",
                    "start": 1622548800,
                    "site_id": "67d4b2dc100e40fcf7b47004",
                    "end": 1622550600,
                    "_id": "68c310ad0636226529c5f22b",
                    "mac": "00:11:22:33:44:55",
                    "expired": false
                },
                {
                    "authorized_by": "api",
                    "start": 1622548800,
                    "site_id": "67d4b2dc100e40fcf7b47004",
                    "end": 1622550600,
                    "_id": "68c310ad0636226529c5f229",
                    "mac": "aa:bb:cc:dd:ee:ff",
                    "expired": true,
                    "unauthorized_by": "api"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let guests = client.guests().list().send().await.unwrap();
        assert_eq!(guests.len(), 2);

        match &guests[0] {
            GuestEntry::Inactive { mac, expired, .. } => {
                assert_eq!(mac, "00:11:22:33:44:55");
                assert!(!expired);
            }
            _ => panic!("Expected Inactive guest entry"),
        }

        match &guests[1] {
            GuestEntry::Inactive {
                mac,
                expired,
                unauthorized_by,
                ..
            } => {
                assert_eq!(mac, "aa:bb:cc:dd:ee:ff");
                assert!(expired);
                assert_eq!(unauthorized_by.as_deref(), Some("api"));
            }
            _ => panic!("Expected Inactive guest entry"),
        }
    }
}

#[tokio::test]
async fn test_list_guests_with_within() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe_and_login(&mock_server, flavor).await;

        let endpoint = api_path(flavor, "/api/s/default/stat/guest");
        let mock = Mock::given(method("GET"))
            .and(path(endpoint.as_str()))
            .and(body_json(json!({"within": 24})));
        let mock = add_auth_headers(mock, flavor);
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": []
        })))
        .mount(&mock_server)
        .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let guests = client
            .guests()
            .list()
            .within_hours(24)
            .send()
            .await
            .unwrap();
        assert!(guests.is_empty());
    }
}

#[tokio::test]
async fn test_unauthorize_guest() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe_and_login(&mock_server, flavor).await;

        let endpoint = api_path(flavor, "/api/s/default/cmd/stamgr");
        let mock = Mock::given(method("POST")).and(path(endpoint.as_str()));
        let mock = add_auth_headers(mock, flavor).and(body_json(json!({
            "cmd": "unauthorize-guest",
            "mac": "00:11:22:33:44:55"
        })));
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": []
        })))
        .mount(&mock_server)
        .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let result = client
            .guests()
            .unauthorize("00:11:22:33:44:55")
            .send()
            .await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_unauthorize_all_guests() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe_and_login(&mock_server, flavor).await;

        // List guests
        let endpoint_list = api_path(flavor, "/api/s/default/stat/guest");
        let mock_list = Mock::given(method("GET")).and(path(endpoint_list.as_str()));
        let mock_list = add_auth_headers(mock_list, flavor);
        mock_list
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
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

        // Unauthorize each guest
        let endpoint_cmd = api_path(flavor, "/api/s/default/cmd/stamgr");

        let mock1 = Mock::given(method("POST")).and(path(endpoint_cmd.as_str()));
        let mock1 = add_auth_headers(mock1, flavor).and(body_json(json!({
            "cmd": "unauthorize-guest",
            "mac": "00:11:22:33:44:55"
        })));
        mock1
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let mock2 = Mock::given(method("POST")).and(path(endpoint_cmd.as_str()));
        let mock2 = add_auth_headers(mock2, flavor).and(body_json(json!({
            "cmd": "unauthorize-guest",
            "mac": "aa:bb:cc:dd:ee:ff"
        })));
        mock2
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "meta": { "rc": "ok" },
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let result = client.guests().unauthorize_all().send().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_guest_api_error() {
    for &flavor in &[TestControllerKind::Network, TestControllerKind::Os] {
        let mock_server = MockServer::start().await;
        setup_probe_and_login(&mock_server, flavor).await;

        let endpoint = api_path(flavor, "/api/s/default/stat/guest");
        let mock = Mock::given(method("GET")).and(path(endpoint.as_str()));
        let mock = add_auth_headers(mock, flavor);
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "error", "msg": "Invalid guest parameters" },
            "data": []
        })))
        .mount(&mock_server)
        .await;

        let client = setup_test_client(&mock_server.uri()).await;
        let result = client.guests().list().send().await;
        assert!(result.is_err());
    }
}
