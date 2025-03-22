use reqwest::Method;

use super::ApiEndpoint;
use crate::{
    AuthorizeGuestRequest, EmptyResponse, GuestConfig, GuestEntry, UnauthorizeGuestRequest,
    UnifiClient, UnifiError,
};

/// Provides methods for managing UniFi wireless guest authorizations.
///
/// This API allows authorizing, listing, and unauthorized wireless guest
/// devices.
pub struct GuestApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for GuestApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> GuestApi<'a> {
    /// Creates a new guest API instance.
    ///
    /// This method is intended for internal use by the UniFi client.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the UniFi client that will be used for API
    ///   requests
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }

    /// Authorizes a guest device for network access.
    ///
    /// This method allows a device to access the network based on the settings
    /// specified in the `config` parameter, such as duration, bandwidth limits,
    /// and data quota.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the guest authorization,
    ///   including MAC address, duration, and bandwidth limits
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns
    /// an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> Result<(), unifi_client::UnifiError> {
    /// use unifi_client::GuestConfig;
    ///
    /// let config = GuestConfig::builder()
    ///     .mac("00:11:22:33:44:55")
    ///     .duration(60) // 60 minutes
    ///     .data_quota(1024) // 1 GB limit
    ///     .build()?;
    ///
    /// let guest = client.guests().authorize(config).await?;
    /// println!("Guest authorized until {}", guest.expires_at());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize(&self, config: GuestConfig) -> Result<GuestEntry, UnifiError> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        let response: Vec<GuestEntry> = client
            .request(
                Method::POST,
                &endpoint,
                Some(AuthorizeGuestRequest::try_from(config)?),
            )
            .await?;

        response
            .first()
            .cloned()
            .ok_or_else(|| UnifiError::ApiError("No authorize guest response received".to_string()))
    }

    /// Lists all guest authorizations within a specified time window.
    ///
    /// # Arguments
    ///
    /// * `within` - Optional time frame in hours to look back for guest
    ///   authorizations (default: 8760 hours / 1 year)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns
    /// an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> Result<(), unifi_client::UnifiError> {
    /// // Get all guest authorizations from the past 24 hours
    /// let guests = client.guests().list(Some(24)).await?;
    ///
    /// for guest in guests {
    ///     println!("Guest {}: expires at {}", guest.mac(), guest.expires_at());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, within: Option<u32>) -> Result<Vec<GuestEntry>, UnifiError> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/guest", site);

        let params = within.map(|hours| serde_json::json!({ "within": hours }));

        let response: Vec<GuestEntry> = client.request(Method::GET, &endpoint, params).await?;

        Ok(response)
    }

    /// Revokes network access for a specific guest device.
    ///
    /// # Arguments
    ///
    /// * `mac` - MAC address of the guest device to unauthorize
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns
    /// an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> Result<(), unifi_client::UnifiError> {
    /// // Unauthorize a specific guest
    /// client.guests().unauthorize("00:11:22:33:44:55").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unauthorize(&self, mac: impl Into<String>) -> Result<(), UnifiError> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);
        let request = UnauthorizeGuestRequest::new(mac);

        let _response: EmptyResponse = client
            .request(Method::POST, &endpoint, Some(request))
            .await?;

        Ok(())
    }

    /// Revokes network access for all authorized guest devices.
    ///
    /// This method retrieves all guest authorizations and then unauthorizes
    /// them one by one. Use with caution as this operation cannot be
    /// undone.
    ///
    /// # Errors
    ///
    /// Returns an error if listing guests fails or if any guest unauthorization
    /// fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> Result<(), unifi_client::UnifiError> {
    /// // Unauthorize all guests in the system
    /// client.guests().unauthorize_all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unauthorize_all(&self) -> Result<(), UnifiError> {
        // Get all guests
        let all_guests = self.list(None).await?;

        // Unauthorize each guest
        for guest in all_guests {
            self.unauthorize(guest.mac()).await?;
        }

        Ok(())
    }
}
