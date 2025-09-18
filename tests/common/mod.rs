#![allow(dead_code)]
use serde_json::json;
use unifi_client::UniFiClient;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockBuilder, MockServer, ResponseTemplate};

pub async fn setup_test_client(mock_server_uri: &str) -> UniFiClient {
    UniFiClient::builder()
        .username("test-user")
        .password("test-password")
        .controller_url(mock_server_uri)
        .site("default")
        .build()
        .await
        .expect("Failed to build UniFiClient")
}

#[derive(Copy, Clone, Debug)]
pub enum TestControllerKind {
    Network,
    Os,
}

impl TestControllerKind {
    pub fn api_base_path(self) -> &'static str {
        match self {
            TestControllerKind::Network => "",
            TestControllerKind::Os => "/proxy/network",
        }
    }

    pub fn login_path(self) -> &'static str {
        match self {
            TestControllerKind::Network => "/api/login",
            TestControllerKind::Os => "/api/auth/login",
        }
    }
}

pub async fn setup_probe(server: &MockServer, kind: TestControllerKind) {
    match kind {
        // For UniFi OS, a HEAD "/" returns 200 which the client uses to detect OS.
        TestControllerKind::Os => {
            Mock::given(method("HEAD"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(200))
                .mount(server)
                .await;
        }
        // For UniFi Network, a HEAD "/" returns 304.
        TestControllerKind::Network => {
            Mock::given(method("HEAD"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(304))
                .mount(server)
                .await;
        }
    }
}

pub async fn setup_probe_and_login(server: &MockServer, kind: TestControllerKind) {
    setup_probe(server, kind).await;

    // Login endpoint differs by kind. Always set a cookie and CSRF header.
    let mut resp = ResponseTemplate::new(200);
    match kind {
        TestControllerKind::Network => {
            resp = resp
                .set_body_json(json!({ "meta": { "rc": "ok" }, "data": [] }))
                .insert_header("set-cookie", "unifises=test-cookie")
        }
        TestControllerKind::Os => {
            resp = resp
                .insert_header("set-cookie", "TOKEN=test-token; path=/")
                .insert_header("x-csrf-token", "test-csrf");
        }
    }

    Mock::given(method("POST"))
        .and(path(kind.login_path()))
        .and(body_json(json!({
            "username": "test-user",
            "password": "test-password"
        })))
        .respond_with(resp)
        .mount(server)
        .await;
}

pub fn api_path(kind: TestControllerKind, endpoint: &str) -> String {
    let mut ep = endpoint.to_string();
    if !ep.starts_with('/') {
        ep.insert(0, '/');
    }
    format!("{}{}", kind.api_base_path(), ep)
}

pub fn csrf_header_for(kind: TestControllerKind) -> Option<(&'static str, &'static str)> {
    match kind {
        TestControllerKind::Network => None,
        TestControllerKind::Os => Some(("x-csrf-token", "test-csrf")),
    }
}

pub fn add_auth_headers(builder: MockBuilder, kind: TestControllerKind) -> MockBuilder {
    let builder = match kind {
        TestControllerKind::Network => builder.and(header("cookie", "unifises=test-cookie")),
        TestControllerKind::Os => builder.and(header("cookie", "TOKEN=test-token")),
    };
    if let Some((k, v)) = csrf_header_for(kind) {
        builder.and(wiremock::matchers::header(k, v))
    } else {
        builder
    }
}
