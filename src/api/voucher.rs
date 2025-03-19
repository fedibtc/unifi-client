use reqwest::Method;

use crate::{UnifiClient, UnifiResult, Voucher};
use super::ApiEndpoint;

/// API for managing vouchers.
pub struct VoucherApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for VoucherApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> VoucherApi<'a> {
    /// Create a new voucher API.
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }
    
    /// Create new vouchers.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of vouchers to create.
    /// * `minutes` - The duration of the vouchers in minutes.
    /// * `note` - An optional note to associate with the vouchers.
    /// * `up` - Optional upload speed limit in Kbps.
    /// * `down` - Optional download speed limit in Kbps.
    /// * `mb_quota` - Optional data quota in MB.
    ///
    /// # Returns
    ///
    /// A vector of created vouchers.
    pub async fn create(
        &self,
        count: u32,
        minutes: u32,
        note: Option<String>,
        up: Option<u32>,
        down: Option<u32>,
        mb_quota: Option<u32>,
    ) -> UnifiResult<Vec<Voucher>> {
        let mut client = self.client.clone();
        
        let mut create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": count,
            "minutes": minutes,
        });
        
        if let Some(note_text) = note {
            create_data["note"] = serde_json::Value::String(note_text);
        }
        
        if let Some(up_limit) = up {
            create_data["up"] = serde_json::Value::Number(up_limit.into());
        }
        
        if let Some(down_limit) = down {
            create_data["down"] = serde_json::Value::Number(down_limit.into());
        }
        
        if let Some(quota) = mb_quota {
            create_data["bytes"] = serde_json::Value::Number((quota * 1024 * 1024).into());
        }
        
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        // First, we create the voucher
        let _: serde_json::Value = client.request(Method::POST, &endpoint, Some(create_data)).await?;
        
        // Then we retrieve the list to get the newly created vouchers
        self.list().await
    }
    
    /// List all vouchers.
    ///
    /// # Returns
    ///
    /// A vector of all vouchers.
    pub async fn list(&self) -> UnifiResult<Vec<Voucher>> {
        let mut client = self.client.clone();
        
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/voucher", site);
        
        let vouchers: Vec<Voucher> = client.request(Method::GET, &endpoint, None::<()>).await?;
        
        Ok(vouchers)
    }
    
    /// Delete a voucher by ID.
    ///
    /// # Arguments
    ///
    /// * `voucher_id` - The ID of the voucher to delete.
    pub async fn delete(&self, voucher_id: &str) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        let delete_data = serde_json::json!({
            "cmd": "delete-voucher",
            "_id": voucher_id,
        });
        
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        let _: serde_json::Value = client.request(Method::POST, &endpoint, Some(delete_data)).await?;
        
        Ok(())
    }
    
    /// Delete all vouchers.
    pub async fn delete_all(&self) -> UnifiResult<()> {
        let vouchers = self.list().await?;
        
        let mut client = self.client.clone();
        
        for voucher in vouchers {
            let delete_data = serde_json::json!({
                "cmd": "delete-voucher",
                "_id": voucher.id,
            });
            
            let site = self.client.site();
            let endpoint = format!("/api/s/{}/cmd/hotspot", site);
            
            let _: serde_json::Value = client.request(Method::POST, &endpoint, Some(delete_data)).await?;
        }
        
        Ok(())
    }
}