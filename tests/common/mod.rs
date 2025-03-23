use unifi_client::{UniFiClient, ClientConfig};

/// Set up a test client with predefined credentials
#[allow(dead_code)]
pub async fn setup_test_client(server_url: &str) -> UniFiClient {
    let config = ClientConfig::builder()
        .controller_url(server_url)
        .username("test-user")
        .password("test-password")
        .site("default")
        .verify_ssl(false)
        .build()
        .unwrap();
    
    let mut client = UniFiClient::new(config);
    
    // Log in
    client.login(None).await.unwrap();
    
    client
}