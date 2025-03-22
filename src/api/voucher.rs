use reqwest::Method;

use super::ApiEndpoint;
use crate::{
    CreateVoucherRequest, CreateVoucherResponse, UnifiClient, UnifiError, UnifiResult, Voucher,
    VoucherConfig,
};

/// Provides methods for managing UniFi wireless guest vouchers.
///
/// This API allows creating, listing, and deleting vouchers used for guest access.
pub struct VoucherApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for VoucherApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> VoucherApi<'a> {
    /// Creates a new voucher API instance.
    ///
    /// This method is intended for internal use by the UniFi client.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the UniFi client that will be used for API requests
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }

    /// Creates new vouchers based on the provided configuration.
    ///
    /// This method generates one or more vouchers according to the settings
    /// specified in the `config` parameter, such as duration, quantity, and bandwidth limits.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the vouchers to be created, including 
    ///   duration, count, bandwidth limits, and other options
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// use unifi_client::VoucherConfig;
    ///
    /// let config = VoucherConfig::builder()
    ///     .note("Conference guests")
    ///     .count(10)
    ///     .duration(1440) // 24 hours in minutes
    ///     .build()?;
    ///
    /// let response = client.vouchers().create(config).await?;
    /// println!("Created {} vouchers", response.create_time);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, config: VoucherConfig) -> UnifiResult<CreateVoucherResponse> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);

        let response: Vec<CreateVoucherResponse> = client
            .request(
                Method::POST,
                &endpoint,
                Some(CreateVoucherRequest::try_from(config)?),
            )
            .await?;

        response
            .first()
            .cloned()
            .ok_or_else(|| UnifiError::ApiError("No create voucher response received".to_string()))
    }

    /// Deletes a specific voucher by its ID.
    ///
    /// # Arguments
    ///
    /// * `voucher_id` - The unique identifier of the voucher to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the voucher cannot be deleted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let voucher_id = "5f8d7c66e4b0abcdef123456";
    /// client.vouchers().delete(voucher_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, voucher_id: &str) -> UnifiResult<()> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);

        let delete_data = serde_json::json!({
            "cmd": "delete-voucher",
            "_id": voucher_id,
        });

        let _: serde_json::Value = client
            .request(Method::POST, &endpoint, Some(delete_data))
            .await?;

        Ok(())
    }

    /// Deletes all existing vouchers.
    ///
    /// This method retrieves all vouchers and then deletes them one by one.
    /// Use with caution as this operation cannot be undone.
    ///
    /// # Errors
    ///
    /// Returns an error if listing vouchers fails or if any voucher deletion fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// // Delete all vouchers in the system
    /// client.vouchers().delete_all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_all(&self) -> UnifiResult<()> {
        // Get all vouchers
        let vouchers = self.list().await?;

        // Delete each voucher
        for voucher in vouchers {
            self.delete(&voucher.id).await?;
        }

        Ok(())
    }

    /// Retrieves vouchers created at a specific time.
    ///
    /// # Arguments
    ///
    /// * `create_time` - Unix timestamp (seconds since epoch) used to filter vouchers
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// // Get vouchers created at a specific time
    /// let timestamp = 1620000000;
    /// let vouchers = client.vouchers().get_by_create_time(timestamp).await?;
    /// println!("Found {} vouchers created at that time", vouchers.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_create_time(&self, create_time: u64) -> UnifiResult<Vec<Voucher>> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/voucher", site);

        let payload = serde_json::json!({
            "create_time": create_time
        });

        let vouchers: Vec<Voucher> = client
            .request(Method::GET, &endpoint, Some(payload))
            .await?;

        Ok(vouchers)
    }

    /// Retrieves all vouchers from the UniFi controller.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let vouchers = client.vouchers().list().await?;
    /// for voucher in vouchers {
    ///     println!("Voucher code: {}, duration: {}", voucher.code, voucher.duration);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> UnifiResult<Vec<Voucher>> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/voucher", site);

        let vouchers: Vec<Voucher> = client.request(Method::GET, &endpoint, None::<()>).await?;

        Ok(vouchers)
    }
}
