use http::{Method, StatusCode};
use serde_json::json;
use wiremock::matchers::{body_json, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

use common::{
    add_auth_headers, api_path, setup_probe, setup_probe_and_login, setup_test_client,
    TestControllerKind,
};
use unifi_client::{UniFiClient, UniFiError};

#[tokio::test]
async fn test_successful_login() -> Result<(), UniFiError> {
    // What it tests: Verifies the full happy path for both controller kinds (Network and OS):
    // probe -> login -> authenticated GET that includes the right auth material
    // (cookie for Network, cookie + CSRF for OS).
    //
    // Why it's valuable: Serves as a smoke test for the core flow and header injection logic. It
    // quickly catches regressions in detection, login, and basic authenticated I/O.
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
        let result = client
            .request_json(Method::GET, "/api/self", None::<()>)
            .await;
        assert!(result.is_ok());
    }
    Ok(())
}

#[tokio::test]
async fn test_failed_login_invalid_credentials() -> Result<(), UniFiError> {
    // What it tests: Login failure with invalid credentials for both controller kinds, including
    // the expected status codes (Network: 400; OS: 403) and error propagation.
    //
    // Why it's valuable: Ensures clear, deterministic error mapping for credential
    // failures so callers can handle auth errors reliably across controller variants.
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
            .accept_invalid_certs(false)
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
    // What it tests: Server-side error during login (HTTP 500) results in an AuthenticationError
    // that includes the failing status.
    //
    // Why it's valuable: Validates that infrastructure outages or upstream faults are surfaced
    // transparently (no silent retries or misleading errors).
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
            .accept_invalid_certs(false)
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
    // What it tests: Login succeeds with 200 but the server omits 'Set-Cookie' → the builder
    // fails with "No cookies received from server" for both controller kinds.
    //
    // Why it's valuable: The cookie is the session anchor. This guards against partially-formed
    // "successful" logins that would later fail with puzzling 401s.
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
            .accept_invalid_certs(false)
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
async fn test_csrf_rotation_on_success_response() -> Result<(), UniFiError> {
    // What it tests: On UniFi OS, the server rotates the CSRF token via 'x-updated-csrf-token' on
    // a success response (200 or 204). The next request must present the new token.
    //
    // Why it's valuable: CSRF rotation can happen on success; this verifies mid-session token
    // update is handled correctly to prevent false 403/401s and stale-header reuse.
    let mock_server = MockServer::start().await;
    setup_probe_and_login(&mock_server, TestControllerKind::Os).await;

    // Use distinct endpoints to prevent matcher collisions between cases.
    let cases: &[(u16, &str)] = &[(200, "/api/self-200"), (204, "/api/self-204")];

    for (status, ep) in cases {
        let endpoint = api_path(TestControllerKind::Os, ep);
        let rotated = format!("rotated-{}", status);

        // 1) First request uses the original CSRF and receives a rotated token in response.
        Mock::given(method("GET"))
            .and(path(endpoint.as_str()))
            .and(header("cookie", "TOKEN=test-token"))
            .and(header("x-csrf-token", "test-csrf"))
            .respond_with(
                ResponseTemplate::new(*status)
                    .insert_header("x-updated-csrf-token", rotated.clone()),
            )
            .mount(&mock_server)
            .await;

        // 2) Second request must use the rotated CSRF.
        Mock::given(method("GET"))
            .and(path(endpoint.as_str()))
            .and(header("cookie", "TOKEN=test-token"))
            .and(header("x-csrf-token", rotated.clone()))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Build a fresh client (per-case) so state starts in a known condition.
        let client = setup_test_client(&mock_server.uri()).await;

        // First call triggers the rotation.
        let resp1 = client.request(Method::GET, ep, None::<()>).await?;
        assert_eq!(resp1.status(), StatusCode::from_u16(*status).unwrap());

        // Second call must present the rotated CSRF to match the second mock (=> 200).
        let resp2 = client.request(Method::GET, ep, None::<()>).await?;
        assert_eq!(resp2.status(), StatusCode::OK);
    }

    Ok(())
}

#[tokio::test]
async fn test_csrf_cleared_after_relogin_without_csrf_header() -> Result<(), UniFiError> {
    // What it tests: After a 401 triggers re-login on UniFi OS, if the new login response does not
    // include 'x-csrf-token', the stored CSRF is cleared and subsequent requests do
    // NOT send the header (and succeed).
    //
    // Why it's valuable: Proves that session state is fully reset across re-auth, avoiding sending
    // an invalid CSRF header that can cause 403s. Prevents "sticky" CSRF bugs.
    let mock_server = MockServer::start().await;
    setup_probe(&mock_server, TestControllerKind::Os).await;

    // Arrange login sequence: first login returns a CSRF token, re-login omits it.
    let login_path = TestControllerKind::Os.login_path();
    let login_body = serde_json::json!({
        "username": "test-user",
        "password": "test-password"
    });

    // Initial login (no cookie yet) seeds CSRF state; remove the mock afterward so re-login uses
    // the dedicated stub below.
    let client = {
        let _initial_login_guard = Mock::given(method("POST"))
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

    let endpoint = api_path(TestControllerKind::Os, "/api/self");

    // 1) The first GET must include the original CSRF and gets forced to 401 to trigger re-auth. We
    //    also keep the cookie matcher to make the intent explicit.
    let first_get_unauthorized = Mock::given(method("GET"))
        .and(path(endpoint.as_str()))
        .and(header("cookie", "TOKEN=test-token"))
        .and(header("x-csrf-token", "test-csrf"))
        .respond_with(ResponseTemplate::new(401));
    first_get_unauthorized.mount(&mock_server).await;

    // 2) Re-login succeeds but does NOT return x-csrf-token. It sets a new cookie
    //    TOKEN=second-token.
    let relogin_without_csrf = Mock::given(method("POST"))
        .and(path(login_path))
        .and(header("cookie", "TOKEN=test-token"))
        .and(body_json(login_body))
        .respond_with(
            ResponseTemplate::new(200).insert_header("set-cookie", "TOKEN=second-token; path=/"),
        )
        .expect(1);
    relogin_without_csrf.mount(&mock_server).await;

    // 3a) Success mock for the retried GET: expects the *new* cookie but does NOT constrain CSRF.
    //     This is the happy-path response (200, rc=ok).
    let retried_get_success = Mock::given(method("GET"))
        .and(path(endpoint.as_str()))
        .and(header("cookie", "TOKEN=second-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": { "rc": "ok" },
            "data": []
        })));
    retried_get_success.mount(&mock_server).await;

    // 3b) Poison pill mock for the retried GET: if a CSRF header is still present alongside
    //     the new cookie, this mock will match and return 418.
    //     We mount this AFTER the success mock so it has higher precedence (wiremock picks
    //     the last matching mock). If the header exists, this one wins → test fails.
    let retried_get_poison_if_csrf_present = Mock::given(method("GET"))
        .and(path(endpoint.as_str()))
        .and(header("cookie", "TOKEN=second-token"))
        .and(header_exists("x-csrf-token"))
        .respond_with(ResponseTemplate::new(418)); // any non-2xx sentinel is fine
    retried_get_poison_if_csrf_present.mount(&mock_server).await;

    // Perform the call (first GET -> 401 -> re-login (no CSRF) -> retry -> 200)
    let result = client
        .request_json(http::Method::GET, "/api/self", None::<()>)
        .await;

    // If CSRF wasn't cleared, the poison mock would have matched and we'd get an error here.
    assert!(
        result.is_ok(),
        "expected success after CSRF cleared, got {result:?}"
    );

    Ok(())
}
