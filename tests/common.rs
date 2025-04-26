use unifi_client::UniFiClient;

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
