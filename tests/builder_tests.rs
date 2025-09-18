use unifi_client::{UniFiClient, UniFiError};

#[tokio::test]
async fn test_config_error() {
    // What it tests: Builder‑time validation of required/structured fields. It covers:
    // (1) a controller URL that fails to parse, and (2) a missing username.
    //
    // Why it's valuable: Fails fast before any network I/O, producing specific
    // ConfigurationError messages that make misconfiguration obvious to callers and reduce
    // debugging time.

    // Test invalid URL
    let err = UniFiClient::builder()
        .controller_url("invalid-url")
        .username("test-user")
        .password("test-password")
        .site("default")
        .accept_invalid_certs(false)
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => {
            assert!(msg.contains("Invalid controller URL"));
        }
        other => panic!("Expected ConfigurationError for invalid URL, got {other:?}"),
    }

    // Test missing username
    let err = UniFiClient::builder()
        .controller_url("https://example.com")
        // No username
        .password("test-password")
        .site("default")
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => assert_eq!(msg, "Username is required"),
        other => panic!("Expected ConfigurationError for missing username, got {other:?}"),
    }
}

#[tokio::test]
async fn test_builder_rejects_empty_username_and_password() {
    // What it tests: The builder rejects empty and whitespace‑only credentials. Both username
    // and password must be present after trimming.
    //
    // Why it's valuable: Enforces a clear, secure contract on input early on, preventing subtle
    // runtime failures or accidental empty credentials from reaching the network layer.

    // Empty username should be rejected before URL parsing/network.
    let err = UniFiClient::builder()
        .controller_url("https://example.com")
        .username("")
        .password("non-empty")
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => assert_eq!(msg, "Username is required"),
        other => panic!("Expected ConfigurationError for username, got {other:?}"),
    }

    // Empty password should be rejected as well.
    let err = UniFiClient::builder()
        .controller_url("https://example.com")
        .username("user")
        .password("")
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => assert_eq!(msg, "Password is required"),
        other => panic!("Expected ConfigurationError for password, got {other:?}"),
    }

    // Whitespace-only username should also be rejected.
    let err = UniFiClient::builder()
        .controller_url("https://example.com")
        .username("   ")
        .password("non-empty")
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => assert_eq!(msg, "Username is required"),
        other => panic!("Expected ConfigurationError for username whitespace, got {other:?}"),
    }

    // Whitespace-only password should also be rejected.
    let err = UniFiClient::builder()
        .controller_url("https://example.com")
        .username("user")
        .password("   ")
        .build()
        .await
        .unwrap_err();
    match err {
        UniFiError::ConfigurationError(msg) => assert_eq!(msg, "Password is required"),
        other => panic!("Expected ConfigurationError for password whitespace, got {other:?}"),
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
#[should_panic(
    expected = "Client must be constructed via `build()` which performs an initial login"
)]
async fn test_default_client_panics_in_debug_without_build() {
    // What it tests: The debug‑only guard that prevents using the inert `UniFiClient::default()`
    // without going through the builder (and initial login). Calling a request method must panic
    // with a clear message in debug builds.
    //
    // Why it's valuable: Guides users toward the correct construction path during development and
    // catches a common misuse early, before it turns into intermittent auth failures at runtime.
    use http::Method;
    let client = UniFiClient::default();
    let _ = client
        .request_json::<()>(Method::GET, "/api/self", None)
        .await;
}
