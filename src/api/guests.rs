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
    /// * `client` - Reference to the UniFi client that will be used for API requests
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
    ///     .duration_minutes(60) // 60 minutes
    ///     .upload_speed_limit_kbps(1024) // 1 Mbps upload
    ///     .download_speed_limit_kbps(2048) // 2 Mbps download
    ///     .data_quota_megabytes(1024) // 1 GB limit
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
    /// let guests = client.guests().list().within_hours(24).send().await?;
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
    /// client
    ///     .guests()
    ///     .unauthorize("00:11:22:33:44:55")
    ///     .send()
    ///     .await?;
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
    unifi_client: UniFiClient,
    client_mac_address: String,
    duration_minutes: Option<u32>,
    upload_speed_limit_kbps: Option<u32>,
    download_speed_limit_kbps: Option<u32>,
    data_quota_megabytes: Option<u64>,
    access_point_mac_address: String,
    captive_portal_timestamp: i64,
    requested_url: Option<String>,
    wifi_network: String,
}

impl AuthorizeGuestBuilder {
    pub(crate) fn new(unifi_client: UniFiClient, client_mac_address: String) -> Self {
        Self {
            unifi_client,
            client_mac_address,
            duration_minutes: None,
            upload_speed_limit_kbps: None,
            download_speed_limit_kbps: None,
            data_quota_megabytes: None,
            access_point_mac_address: "00:00:00:00:00:00".to_string(),
            captive_portal_timestamp: 0,
            requested_url: None,
            wifi_network: "".to_string(),
        }
    }

    pub fn duration_minutes(mut self, duration_minutes: u32) -> Self {
        self.duration_minutes = Some(duration_minutes);
        self
    }

    pub fn upload_speed_limit_kbps(mut self, upload_speed_limit_kbps: u32) -> Self {
        self.upload_speed_limit_kbps = Some(upload_speed_limit_kbps);
        self
    }

    pub fn download_speed_limit_kbps(mut self, download_speed_limit_kbps: u32) -> Self {
        self.download_speed_limit_kbps = Some(download_speed_limit_kbps);
        self
    }

    pub fn data_quota_megabytes(mut self, data_quota_megabytes: u64) -> Self {
        self.data_quota_megabytes = Some(data_quota_megabytes);
        self
    }

    pub fn access_point_mac_address(mut self, access_point_mac_address: impl Into<String>) -> Self {
        self.access_point_mac_address = access_point_mac_address.into();
        self
    }

    pub fn captive_portal_timestamp(mut self, captive_portal_timestamp: i64) -> Self {
        self.captive_portal_timestamp = captive_portal_timestamp;
        self
    }

    pub fn requested_url(mut self, requested_url: impl Into<String>) -> Self {
        self.requested_url = Some(requested_url.into());
        self
    }

    pub fn wifi_network(mut self, wifi_network: impl Into<String>) -> Self {
        self.wifi_network = wifi_network.into();
        self
    }

    pub async fn send(self) -> UniFiResult<models::guests::GuestEntry> {
        let site = self.unifi_client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        let request = models::guests::AuthorizeGuestRequest {
            cmd: "authorize-guest".to_string(),
            mac: self.client_mac_address,
            minutes: self.duration_minutes,
            up: self.upload_speed_limit_kbps,
            down: self.download_speed_limit_kbps,
            bytes: self.data_quota_megabytes,
            ap_mac: self.access_point_mac_address,
        };

        let response: Vec<models::guests::GuestEntry> = self
            .unifi_client
            .request(Method::POST, &endpoint, Some(request))
            .await?;

        response
            .into_iter()
            .next()
            .ok_or_else(|| UniFiError::ApiError("No authorize guest response received".to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ListGuestsBuilder {
    client: UniFiClient,
    within_hours: Option<u32>,
}

impl ListGuestsBuilder {
    pub(crate) fn new(client: UniFiClient) -> Self {
        Self {
            client,
            within_hours: None,
        }
    }

    /// Limit results to the past `hours` hours.
    /// Defaults to “all time” if not set.
    pub fn within_hours(mut self, hours: u32) -> Self {
        self.within_hours = Some(hours);
        self
    }

    pub async fn send(self) -> UniFiResult<Vec<models::guests::GuestEntry>> {
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/guest", site);

        let params = self
            .within_hours
            .map(|hours| serde_json::json!({ "within": hours }));

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
        let request = models::guests::UnauthorizeGuestRequest::new(self.mac);

        self.client
            .request(Method::POST, &endpoint, Some(request))
            .await
            .map(|_: EmptyResponse| ())
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
