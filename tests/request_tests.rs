use http::Method;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

use common::{api_path, setup_probe, setup_probe_and_login, setup_test_client, TestControllerKind};
use unifi_client::UniFiError;

#[tokio::test]
async fn test_invalid_endpoint_rejected() -> Result<(), UniFiError> {
    // What it tests: The client rejects endpoints containing a query string (or fragment) and
    // surfaces a UniFiError::InvalidEndpoint instead of silently mangling the URL.
    // Although this runs against an OS mock, the invariant is endpoint-shape only and applies
    // uniformly across controller kinds.
    //
    // Why it's valuable: Prevents subtle bugs where `?query` would be pushed as path segments.
    // It forces callers to follow the library contract (“path only”), fails fast at the callsite,
    // and avoids confusing controller-side 404/401s originating from malformed paths.
    let mock_server = MockServer::start().await;

    setup_probe_and_login(&mock_server, TestControllerKind::Os).await;

    let client = setup_test_client(&mock_server.uri()).await;
    match client
        .request(Method::GET, "/api/self?foo=bar", None::<()>)
        .await
    {
        Err(UniFiError::InvalidEndpoint(msg)) => {
            assert!(msg.contains("endpoint must not include query"));
        }
        other => panic!("expected InvalidEndpoint error for query-bearing path, got {other:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_request_retries_once_on_403_os() -> Result<(), UniFiError> {
    // What it tests: On UniFi OS, a 403 on the first request triggers exactly one re-login and
    // a single retry using the refreshed cookie + rotated CSRF. The test asserts both the success
    // of the retried request and the precise number of attempts (two GETs, two POSTs to login).
    //
    // Why it's valuable: Validates the critical re-auth policy for OS (treat 403 as re-auth
    // signal), ensures we neither skip the retry (leading to spurious failures) nor over-retry
    // (risking loops / traffic storms), and confirms header/cookie rotation is wired correctly.
    let mock_server = MockServer::start().await;
    setup_probe(&mock_server, TestControllerKind::Os).await;

    let login_path = TestControllerKind::Os.login_path();
    let login_body = json!({
        "username": "test-user",
        "password": "test-password"
    });

    let client = {
        let _initial_login = Mock::given(method("POST"))
            .and(path(login_path))
            .and(body_json(login_body.clone()))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("set-cookie", "TOKEN=test-token; path=/")
                    .insert_header("x-csrf-token", "test-csrf"),
            )
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;

        setup_test_client(&mock_server.uri()).await
    };

    let endpoint_path = api_path(TestControllerKind::Os, "/api/self");

    // First attempt returns 403 with the original credentials, forcing a re-authentication.
    Mock::given(method("GET"))
        .and(path(endpoint_path.as_str()))
        .and(header("cookie", "TOKEN=test-token"))
        .and(header("x-csrf-token", "test-csrf"))
        .respond_with(ResponseTemplate::new(403))
        .expect(1)
        .mount(&mock_server)
        .await;

    // The client should retry exactly once by re-authenticating.
    Mock::given(method("POST"))
        .and(path(login_path))
        .and(header("cookie", "TOKEN=test-token"))
        .and(body_json(login_body.clone()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "TOKEN=second-token; path=/")
                .insert_header("x-csrf-token", "rotated-csrf"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // The retried request must succeed using the rotated credentials.
    Mock::given(method("GET"))
        .and(path(endpoint_path.as_str()))
        .and(header("cookie", "TOKEN=second-token"))
        .and(header("x-csrf-token", "rotated-csrf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": []
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Execute the request: the client will perform the initial attempt, see the 403, re-login once,
    // and retry successfully.
    let result = client
        .request_json(Method::GET, "/api/self", None::<()>)
        .await?;
    assert!(result.is_array());

    let requests = mock_server
        .received_requests()
        .await
        .expect("failed to read recorded requests");

    let get_attempts = requests
        .iter()
        .filter(|req| req.method == Method::GET && req.url.path() == endpoint_path)
        .count();
    assert_eq!(
        get_attempts, 2,
        "expected exactly two GET attempts (original + retry), saw {get_attempts}"
    );

    let login_attempts = requests
        .iter()
        .filter(|req| req.method == Method::POST && req.url.path() == login_path)
        .count();
    assert_eq!(
        login_attempts, 2,
        "expected initial login plus one re-authentication, saw {login_attempts}"
    );

    Ok(())
}
