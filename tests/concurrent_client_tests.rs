use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Barrier;
use unifi_client::UniFiClient;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[tokio::test]
async fn test_concurrent_client_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Set up counter for login attempts
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = Arc::clone(&login_count);

    // Set up WireMock server
    let mock_server = MockServer::start().await;

    // Mock successful login
    Mock::given(method("POST"))
        .and(path("/api/login"))
        .and(body_json(json!({
            "username": "admin",
            "password": "password"
        })))
        .respond_with(move |_: &Request| {
            // Count login attempts
            login_count_clone.fetch_add(1, Ordering::SeqCst);

            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "meta": { "rc": "ok" },
                    "data": []
                }))
                .insert_header("set-cookie", "unifises=test-cookie")
        })
        .expect(2)
        .mount(&mock_server)
        .await;

    // Set up guest authorize mock: first call 401, then 200 with cookie asserted.
    let authz_calls = Arc::new(AtomicUsize::new(0));
    let authz_calls_clone = Arc::clone(&authz_calls);
    Mock::given(method("POST"))
        .and(path("/api/s/default/cmd/stamgr"))
        .and(body_json(json!({
            "cmd": "authorize-guest",
            "mac": "00:11:22:33:44:55",
            "minutes": 30,
            "ap_mac": "00:00:00:00:00:00",
        })))
        .respond_with(move |req: &Request| {
            let prev = authz_calls_clone.fetch_add(1, Ordering::SeqCst);
            if prev == 0 {
                ResponseTemplate::new(401)
            } else {
                // After re-auth, cookie should be present.
                assert_eq!(
                    req.headers.get("cookie").and_then(|v| v.to_str().ok()),
                    Some("unifises=test-cookie")
                );
                ResponseTemplate::new(200).set_body_json(json!({
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
                }))
            }
        })
        .expect(2)
        .mount(&mock_server)
        .await;

    // Set up guest list mock
    let list_calls = Arc::new(AtomicUsize::new(0));
    let list_calls_clone = Arc::clone(&list_calls);
    Mock::given(method("GET"))
        .and(path("/api/s/default/stat/guest"))
        .respond_with(move |req: &Request| {
            let prev = list_calls_clone.fetch_add(1, Ordering::SeqCst);
            if prev == 0 {
                ResponseTemplate::new(401)
            } else {
                assert_eq!(
                    req.headers.get("cookie").and_then(|v| v.to_str().ok()),
                    Some("unifises=test-cookie")
                );
                ResponseTemplate::new(200).set_body_json(json!({
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
                }))
            }
        })
        .expect(3) // two during concurrent tasks + one after
        .mount(&mock_server)
        .await;

    // Build the client
    let client = UniFiClient::builder()
        .controller_url(&mock_server.uri())
        .username("admin")
        .password("password")
        .build()
        .await?;

    // Verify the client only logged in once during build
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        1,
        "Client should authenticate once during build"
    );

    // Create a barrier to ensure both tasks start at the same time
    let barrier = Arc::new(Barrier::new(2));

    // Create two clones of the client for concurrent tasks
    let client1 = client.clone();
    let client2 = client.clone();
    let barrier1 = Arc::clone(&barrier);
    let barrier2 = Arc::clone(&barrier);

    // Launch two concurrent tasks
    let task1 = tokio::spawn(async move {
        barrier1.wait().await;
        client1
            .guests()
            .authorize("00:11:22:33:44:55")
            .duration_minutes(30)
            .send()
            .await
    });

    let task2 = tokio::spawn(async move {
        barrier2.wait().await;
        client2.guests().list().send().await
    });

    // Wait for both tasks to complete
    let authorize_guest_result = task1.await?;
    let list_guests_result = task2.await?;

    // Verify both tasks succeeded
    let authorize_guest = authorize_guest_result?;
    let list_guests = list_guests_result?;

    // Verify we got the expected mock data
    assert_eq!(authorize_guest.mac(), "00:11:22:33:44:55");
    assert_eq!(list_guests.len(), 2, "Should have received 2 guests");

    // Verify login was only called once (initial authentication)
    // This confirms the auth state is properly shared
    let concurrent_login_count = login_count.load(Ordering::SeqCst);
    assert_eq!(
        concurrent_login_count, 2,
        "Should re-login exactly once on 401"
    );

    // Verify the authentication is only happening when needed
    // Make one more request to confirm auth is reused
    client.guests().list().send().await?;

    // Verify no additional logins were required
    let final_login_count = login_count.load(Ordering::SeqCst);
    assert_eq!(
        final_login_count, 2,
        "No additional logins should occur beyond one re-auth"
    );

    Ok(())
}
