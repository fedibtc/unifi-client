use serde_json::Value;
use unifi_client::{UniFiClient, UniFiResult};

pub struct SitesValidator {
    client: UniFiClient,
}

impl SitesValidator {
    pub fn new(client: UniFiClient) -> Self {
        Self { client }
    }

    async fn validate_site_info(&self) -> UniFiResult<()> {
        let client = self.client.clone();
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

    async fn validate_list_sites(&self) -> UniFiResult<()> {
        let client = self.client.clone();
        let endpoint = format!("/api/stat/sites");

        let sites: Value = client.raw_request("GET", &endpoint, None::<()>).await?;

        if let Some(sites_array) = sites.as_array() {
            if !sites_array.is_empty() {
                println!("✅ List sites test passed");
            } else {
                println!("No sites found.");
            }
        } else {
            println!("❌ List sites test failed: could not parse site list");
        }

        Ok(())
    }

    pub async fn run_all_validations(&self) -> UniFiResult<()> {
        println!("Running site validator...");
        self.validate_site_info().await?;
        self.validate_list_sites().await?;
        Ok(())
    }
}
