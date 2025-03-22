use reqwest::Method;

use super::ApiEndpoint;
use crate::{Site, SiteStats, UnifiClient, UnifiError, UnifiResult};

/// Provides methods for managing UniFi Controller sites.
///
/// This API allows creating, listing, updating, and deleting sites within the UniFi system,
/// as well as retrieving site statistics.
pub struct SiteApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for SiteApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> SiteApi<'a> {
    /// Creates a new site API instance.
    ///
    /// This method is intended for internal use by the UniFi client.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the UniFi client that will be used for API requests
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }

    /// Retrieves all sites from the UniFi controller.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or if the UniFi controller returns an error response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let sites = client.sites().list().await?;
    /// for site in sites {
    ///     println!("Site: {}", site);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> UnifiResult<Vec<Site>> {
        let mut client = self.client.clone();

        let endpoint = "/api/self/sites";

        let sites: Vec<Site> = client.request(Method::GET, endpoint, None::<()>).await?;

        Ok(sites)
    }

    /// Retrieves a specific site by its ID.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The unique identifier of the site to retrieve
    ///
    /// # Errors
    ///
    /// Returns `UnifiError::SiteNotFound` if the site does not exist or is not accessible.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let site = client.sites().get("5f8d7c66e4b0abcdef123456").await?;
    /// println!("Retrieved site: {}", site);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, site_id: &str) -> UnifiResult<Site> {
        let sites = self.list().await?;

        sites
            .into_iter()
            .find(|site| site.id == site_id)
            .ok_or_else(|| UnifiError::SiteNotFound(site_id.to_string()))
    }

    /// Retrieves a specific site by its name or description.
    ///
    /// This method searches both the site name (used in API calls) and the human-readable
    /// description for a match.
    ///
    /// # Arguments
    ///
    /// * `name` - The name or description of the site to retrieve
    ///
    /// # Errors
    ///
    /// Returns `UnifiError::SiteNotFound` if no site matches the provided name or description.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// // Find site either by name or description
    /// let site = client.sites().get_by_name("Main Office").await?;
    /// println!("Retrieved site: {}", site);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_name(&self, name: &str) -> UnifiResult<Site> {
        let sites = self.list().await?;

        sites
            .into_iter()
            .find(|site| site.name == name || site.desc == name)
            .ok_or_else(|| UnifiError::SiteNotFound(name.to_string()))
    }

    /// Creates a new site on the UniFi controller.
    ///
    /// # Arguments
    ///
    /// * `name` - The site name to use in API calls (should be URL-friendly)
    /// * `description` - The human-readable description of the site
    ///
    /// # Errors
    ///
    /// Returns an error if the site creation fails or if a site with the same name already exists.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let new_site = client.sites().create("branch-office", "Branch Office").await?;
    /// println!("Created new site: {}", new_site);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, name: &str, description: &str) -> UnifiResult<Site> {
        let mut client = self.client.clone();

        let create_data = serde_json::json!({
            "cmd": "add-site",
            "name": name,
            "desc": description
        });

        let endpoint = "/api/s/default/cmd/sitemgr";

        let _: serde_json::Value = client
            .request(Method::POST, endpoint, Some(create_data))
            .await?;

        // The API doesn't return the created site, so we need to fetch it
        self.get_by_name(name).await
    }

    /// Updates an existing site's description.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The ID of the site to update
    /// * `description` - The new description for the site
    ///
    /// # Errors
    ///
    /// Returns `UnifiError::SiteNotFound` if the site does not exist or is not accessible.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let updated_site = client.sites()
    ///     .update("5f8d7c66e4b0abcdef123456", "Updated Description")
    ///     .await?;
    /// println!("Updated site: {}", updated_site);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(&self, site_id: &str, description: &str) -> UnifiResult<Site> {
        let mut client = self.client.clone();

        // First, get the current site to ensure it exists
        let _ = self.get(site_id).await?;

        let update_data = serde_json::json!({
            "cmd": "update-site",
            "site_id": site_id,
            "desc": description
        });

        let endpoint = "/api/s/default/cmd/sitemgr";

        let _: serde_json::Value = client
            .request(Method::POST, endpoint, Some(update_data))
            .await?;

        // The API doesn't return the updated site, so we need to fetch it
        self.get(site_id).await
    }

    /// Deletes a site from the UniFi controller.
    ///
    /// Use with caution as this operation cannot be undone.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The ID of the site to delete
    ///
    /// # Errors
    ///
    /// Returns `UnifiError::SiteNotFound` if the site does not exist or is not accessible.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// client.sites().delete("5f8d7c66e4b0abcdef123456").await?;
    /// println!("Site deleted successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, site_id: &str) -> UnifiResult<()> {
        let mut client = self.client.clone();

        // First, get the current site to ensure it exists
        let _ = self.get(site_id).await?;

        let delete_data = serde_json::json!({
            "cmd": "delete-site",
            "site_id": site_id
        });

        let endpoint = "/api/s/default/cmd/sitemgr";

        let _: serde_json::Value = client
            .request(Method::POST, endpoint, Some(delete_data))
            .await?;

        Ok(())
    }

    /// Sets the specified site as the default for this client instance.
    ///
    /// This changes which site is used for subsequent API calls with the returned client.
    ///
    /// # Arguments
    ///
    /// * `site` - The site to set as default
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let sites = client.sites().list().await?;
    /// let first_site = &sites[0];
    /// 
    /// // Create a new client instance with a different default site
    /// let new_client = client.sites().set_as_default(first_site);
    /// 
    /// // Subsequent calls with new_client will use the specified site
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_as_default(&self, site: &Site) -> UnifiClient {
        let mut new_client = self.client.clone();
        new_client.config.site = site.name.clone();
        new_client
    }

    /// Retrieves statistics for the current site.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(client: &unifi_client::UnifiClient) -> unifi_client::UnifiResult<()> {
    /// let stats = client.sites().stats().await?;
    /// println!("Site has {} access points and {} clients", stats.num_ap, stats.num_user);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> UnifiResult<SiteStats> {
        let mut client = self.client.clone();

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/health", site);

        let stats: Vec<SiteStats> = client.request(Method::GET, &endpoint, None::<()>).await?;

        stats
            .into_iter()
            .next()
            .ok_or_else(|| UnifiError::ApiError("No site statistics available".to_string()))
    }
}
