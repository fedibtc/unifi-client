use reqwest::Method;

use super::ApiEndpoint;
use crate::{
    AuthorizeGuestRequest, EmptyResponse, GuestConfig, GuestEntry, UnauthorizeGuestRequest,
    UnifiClient, UnifiError,
};

/// API for managing guest authorizations.
pub struct GuestApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for GuestApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> GuestApi<'a> {
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }

    /// Authorize a guest device.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the guest authorization
    ///
    /// # Returns
    ///
    /// * `Ok(GuestEntry)` if the guest was authorized successfully
    /// * `Err(UnifiError)` if the authorization failed
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

    /// List guest authorizations within a specified time window
    ///
    /// # Arguments
    ///
    /// * `within` - Optional time frame in hours to look back for guest
    ///   authorizations (default: 8760 hours / 1 year)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<GuestEntry>)` containing both active and expired guest
    ///   authorizations
    /// * `Err(UnifiError)` if the request failed
    pub async fn list(&self, within: Option<u32>) -> Result<Vec<GuestEntry>, UnifiError> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/guest", site);

        let params = within.map(|hours| serde_json::json!({ "within": hours }));

        let response: Vec<GuestEntry> = client.request(Method::GET, &endpoint, params).await?;

        Ok(response)
    }

    /// Unauthorize a guest device.
    ///
    /// # Arguments
    ///
    /// * `mac` - MAC address of the guest device to unauthorize
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the guest was unauthorized successfully
    /// * `Err(UnifiError)` if the unauthorization failed
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

    /// Unauthorize all guests.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all guests were unauthorized successfully
    /// * `Err(UnifiError)` if the unauthorization failed
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
