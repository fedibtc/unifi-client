use reqwest::Method;

use super::ApiEndpoint;
use crate::{
    CreateVoucherRequest, CreateVoucherResponse, UnifiClient, UnifiError, UnifiResult, Voucher,
    VoucherExpireUnit,
};

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
    /// * `minutes` - The duration the voucher is valid after activation in
    ///   minutes.
    /// * `note` - An optional note to associate with the vouchers.
    /// * `up` - Optional upload speed limit in Kbps.
    /// * `down` - Optional download speed limit in Kbps.
    /// * `mb_quota` - Optional data transfer limit in MB.
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
        transfer_limit: Option<u32>,
    ) -> UnifiResult<CreateVoucherResponse> {
        let mut client = self.client.clone();

        let create_data = CreateVoucherRequest {
            cmd: "create-voucher".to_string(),
            n: count,
            expire: Some(minutes),
            expire_number: Some(1),
            expire_unit: Some(VoucherExpireUnit::Minutes),
            quota: Some(1),
            note,
            up,
            down,
            bytes: transfer_limit,
        };

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);

        // Create the voucher and get the response
        let response: Vec<CreateVoucherResponse> = client
            .request(Method::POST, &endpoint, Some(create_data))
            .await?;

        // Extract the first (and likely only) create voucher response
        match response.first() {
            Some(create_response) => Ok(create_response.clone()),
            None => Err(UnifiError::ApiError(
                "No create voucher response received".to_string(),
            )),
        }
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

        let _: serde_json::Value = client
            .request(Method::POST, &endpoint, Some(delete_data))
            .await?;

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

            let _: serde_json::Value = client
                .request(Method::POST, &endpoint, Some(delete_data))
                .await?;
        }

        Ok(())
    }

    /// Get vouchers filtered by creation time
    ///
    /// # Arguments
    ///
    /// * `create_time` - Unix timestamp to filter vouchers by creation time
    ///
    /// # Returns
    ///
    /// A vector of vouchers created at the specified time
    pub async fn get_by_create_time(&self, create_time: u64) -> UnifiResult<Vec<Voucher>> {
        let mut client = self.client.clone();

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/voucher", site);

        // Create payload with create_time parameter using json! macro
        let payload = serde_json::json!({
            "create_time": create_time
        });

        // Make the request
        let vouchers: Vec<Voucher> = client
            .request(Method::GET, &endpoint, Some(payload))
            .await?;

        Ok(vouchers)
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

        let vouchers: Vec<Voucher> = client.request(Method::GET, &endpoint,
        None::<()>).await?;

        Ok(vouchers)
    }
}
