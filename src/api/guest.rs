use reqwest::Method;

use crate::models::EmptyResponse;
use crate::{models, UniFiClient, UniFiError, UniFiResult};

/// Provides methods for managing UniFi wireless guest authorizations.
///
/// This API allows authorizing, listing, and unauthorized wireless guest
/// devices.
#[derive(Debug)]
pub struct GuestHandler {
    client: UniFiClient,
}

impl GuestHandler {
    /// Creates a new guest API instance.
    ///
    /// This method is intended for internal use by the UniFi client.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the UniFi client that will be used for API
    ///   requests
    pub(crate) fn new(client: UniFiClient) -> Self {
        Self { client }
    }

    /// Authorizes a guest device for network access.
    ///
    /// This method allows a device to access the network based on the settings
    /// specified in the builder methods, such as duration, bandwidth limits,
    /// and data quota.
    ///
    /// # Arguments
    ///
    /// * `mac` - The MAC address of the guest device to authorize.
    ///
    /// # Returns
    ///
    /// Returns an `AuthorizeGuestBuilder` instance, which allows for setting
    /// optional parameters before sending the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::UniFiClient;
    /// #
    /// # async fn example(client: &UniFiClient) -> Result<(), unifi_client::UniFiError> {
    /// let guest = client
    ///     .guests()
    ///     .authorize("00:11:22:33:44:55") // MAC address of the guest
    ///     .duration(60) // 60 minutes
    ///     .up(1024) // 1 Mbps upload
    ///     .down(2048) // 2 Mbps download
    ///     .data_quota(1024) // 1 GB limit
    ///     .send() // Send the request
    ///     .await?;
    ///
    /// println!("Guest authorized until {}", guest.expires_at());
    /// # Ok(())
    /// # }
    /// ```
    pub fn authorize(&self, mac: impl Into<String>) -> AuthorizeGuestBuilder {
        AuthorizeGuestBuilder::new(self.client.clone(), mac.into())
    }

    /// Lists all guest authorizations within a specified time window.
    ///
    /// # Returns
    ///
    /// Returns a `ListGuestsBuilder` instance, which allows for setting
    /// optional parameters before sending the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::UniFiClient;
    /// #
    /// # async fn example(client: &UniFiClient) -> Result<(), unifi_client::UniFiError> {
    /// // Get all guest authorizations from the past 24 hours
    /// let guests = client.guests().list().within(24).send().await?;
    ///
    /// for guest in guests {
    ///     println!("Guest {}: expires at {}", guest.mac(), guest.expires_at());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(&self) -> ListGuestsBuilder {
        ListGuestsBuilder::new(self.client.clone())
    }

    /// Revokes network access for a specific guest device.
    ///
    /// # Arguments
    ///
    /// * `mac` - MAC address of the guest device to unauthorize
    ///
    /// # Returns
    ///
    /// Returns an `UnauthorizeGuestBuilder` instance.  Call `.send()` on this
    /// builder to execute the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::UniFiClient;
    /// #
    /// # async fn example(client: &UniFiClient) -> Result<(), unifi_client::UniFiError> {
    /// // Unauthorize a specific guest
    /// client.guests().unauthorize("00:11:22:33:44:55").send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unauthorize(&self, mac: impl Into<String>) -> UnauthorizeGuestBuilder {
        UnauthorizeGuestBuilder::new(self.client.clone(), mac.into())
    }

    /// Revokes network access for all authorized guest devices.
    ///
    /// This method retrieves all guest authorizations and then unauthorizes
    /// them one by one. Use with caution as this operation cannot be
    /// undone.
    ///
    /// # Returns
    ///
    /// Returns an `UnauthorizeAllGuestsBuilder` instance. Call `.send()` on
    /// this builder to execute the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use unifi_client::UniFiClient;
    /// #
    /// # async fn example(client: &UniFiClient) -> Result<(), unifi_client::UniFiError> {
    /// // Unauthorize all guests in the system
    /// client.guests().unauthorize_all().send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unauthorize_all(&self) -> UnauthorizeAllGuestsBuilder {
        UnauthorizeAllGuestsBuilder::new(self.client.clone())
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizeGuestBuilder {
    client: UniFiClient,
    mac: String,
    duration: Option<u32>,
    up: Option<u32>,
    down: Option<u32>,
    data_quota: Option<u64>,
    ap_mac: Option<String>,
}

impl AuthorizeGuestBuilder {
    pub(crate) fn new(client: UniFiClient, mac: String) -> Self {
        Self {
            client,
            mac,
            duration: None,
            up: None,
            down: None,
            data_quota: None,
            ap_mac: None,
        }
    }

    pub fn duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn up(mut self, up: u32) -> Self {
        self.up = Some(up);
        self
    }

    pub fn down(mut self, down: u32) -> Self {
        self.down = Some(down);
        self
    }

    pub fn data_quota(mut self, data_quota: u64) -> Self {
        self.data_quota = Some(data_quota);
        self
    }

    pub fn ap_mac(mut self, ap_mac: impl Into<String>) -> Self {
        self.ap_mac = Some(ap_mac.into());
        self
    }

    pub async fn send(self) -> UniFiResult<models::guest::GuestEntry> {
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        let request = models::guest::AuthorizeGuestRequest {
            cmd: "authorize-guest".to_string(),
            mac: self.mac,
            minutes: self.duration,
            up: self.up,
            down: self.down,
            bytes: self.data_quota,
            ap_mac: self.ap_mac,
        };

        let response: Vec<models::guest::GuestEntry> =
            self.client.request(Method::POST, &endpoint, Some(request)).await?;

        response
            .into_iter()
            .next()
            .ok_or_else(|| UniFiError::ApiError("No authorize guest response received".to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ListGuestsBuilder {
    client: UniFiClient,
    within: Option<u32>,
}

impl ListGuestsBuilder {
    pub(crate) fn new(client: UniFiClient) -> Self {
        Self {
            client,
            within: None,
        }
    }
    pub fn within(mut self, within: u32) -> Self {
        self.within = Some(within);
        self
    }

    pub async fn send(self) -> UniFiResult<Vec<models::guest::GuestEntry>> {
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/guest", site);
        let params = self.within.map(|hours| serde_json::json!({ "within": hours }));

        self.client.request(Method::GET, &endpoint, params).await
    }
}

#[derive(Debug, Clone)]
pub struct UnauthorizeGuestBuilder {
    client: UniFiClient,
    mac: String,
}

impl UnauthorizeGuestBuilder {
    pub(crate) fn new(client: UniFiClient, mac: String) -> Self {
        Self { client, mac }
    }

    pub async fn send(self) -> UniFiResult<()> {
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);
        let request = models::guest::UnauthorizeGuestRequest::new(self.mac);

        self.client.request(Method::POST, &endpoint, Some(request)).await.map(|_: EmptyResponse| ())
    }
}

#[derive(Debug, Clone)]
pub struct UnauthorizeAllGuestsBuilder {
    client: UniFiClient,
}

impl UnauthorizeAllGuestsBuilder {
    pub(crate) fn new(client: UniFiClient) -> Self {
        Self { client }
    }

    pub async fn send(self) -> UniFiResult<()> {
        let all_guests = self.client.guests().list().send().await?;
        for guest in all_guests {
            self.client.guests().unauthorize(guest.mac()).send().await?
        }
        Ok(())
    }
}
