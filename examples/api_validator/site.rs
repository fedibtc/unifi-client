use serde_json::Value;
use unifi_client::{UnifiClient, UnifiResult};

pub struct SiteValidator {
    client: UnifiClient,
}

impl SiteValidator {
    pub fn new(client: UnifiClient) -> Self {
        Self { client }
    }

    async fn validate_site_info(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/sysinfo", site);
        
        let site_info: Value = client.raw_request("GET", &endpoint, None::<()>).await?;
        
        // Validate site info structure
        if let Some(info) = site_info.as_array().and_then(|v| v.first()) {
            if info["name"].is_string() {
                println!("✅ Site info test passed");
            } else {
                println!("❌ Site info test failed: missing name field");
            }
        }
        
        Ok(())
    }

    pub async fn run_all_validations(&self) -> UnifiResult<()> {
        self.validate_site_info().await?;
        Ok(())
    }
}