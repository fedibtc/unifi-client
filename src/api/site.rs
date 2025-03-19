use reqwest::Method;

use crate::{UnifiClient, UnifiResult, UnifiError, Site, SiteStats};
use super::ApiEndpoint;

/// API for managing sites.
pub struct SiteApi<'a> {
    client: &'a UnifiClient,
}

impl<'a> ApiEndpoint for SiteApi<'a> {
    fn client(&self) -> &UnifiClient {
        self.client
    }
}

impl<'a> SiteApi<'a> {
    /// Create a new site API.
    pub(crate) fn new(client: &'a UnifiClient) -> Self {
        Self { client }
    }
    
    /// List all sites.
    ///
    /// # Returns
    ///
    /// A vector of all sites.
    pub async fn list(&self) -> UnifiResult<Vec<Site>> {
        let mut client = self.client.clone();
        
        let endpoint = "/api/self/sites";
        
        let sites: Vec<Site> = client.request(Method::GET, endpoint, None::<()>).await?;
        
        Ok(sites)
    }
    
    /// Get a specific site by ID.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The ID of the site to get.
    ///
    /// # Returns
    ///
    /// The site if found.
    pub async fn get(&self, site_id: &str) -> UnifiResult<Site> {
        let sites = self.list().await?;
        
        sites.into_iter()
            .find(|site| site.id == site_id)
            .ok_or_else(|| UnifiError::SiteNotFound(site_id.to_string()))
    }
    
    /// Get a specific site by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name or description of the site to get.
    ///
    /// # Returns
    ///
    /// The site if found.
    pub async fn get_by_name(&self, name: &str) -> UnifiResult<Site> {
        let sites = self.list().await?;
        
        sites.into_iter()
            .find(|site| site.name == name || site.desc == name)
            .ok_or_else(|| UnifiError::SiteNotFound(name.to_string()))
    }
    
    /// Create a new site.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the site (used in API calls).
    /// * `description` - The human-readable description of the site.
    ///
    /// # Returns
    ///
    /// The newly created site.
    pub async fn create(&self, name: &str, description: &str) -> UnifiResult<Site> {
        let mut client = self.client.clone();
        
        let create_data = serde_json::json!({
            "cmd": "add-site",
            "name": name,
            "desc": description
        });
        
        let endpoint = "/api/s/default/cmd/sitemgr";
        
        let _: serde_json::Value = client.request(Method::POST, endpoint, Some(create_data)).await?;
        
        // The API doesn't return the created site, so we need to fetch it
        self.get_by_name(name).await
    }
    
    /// Update a site.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The ID of the site to update.
    /// * `description` - The new description for the site.
    ///
    /// # Returns
    ///
    /// The updated site.
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
        
        let _: serde_json::Value = client.request(Method::POST, endpoint, Some(update_data)).await?;
        
        // The API doesn't return the updated site, so we need to fetch it
        self.get(site_id).await
    }
    
    /// Delete a site.
    ///
    /// # Arguments
    ///
    /// * `site_id` - The ID of the site to delete.
    ///
    /// # Returns
    ///
    /// Success or error.
    pub async fn delete(&self, site_id: &str) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        // First, get the current site to ensure it exists
        let _ = self.get(site_id).await?;
        
        let delete_data = serde_json::json!({
            "cmd": "delete-site",
            "site_id": site_id
        });
        
        let endpoint = "/api/s/default/cmd/sitemgr";
        
        let _: serde_json::Value = client.request(Method::POST, endpoint, Some(delete_data)).await?;
        
        Ok(())
    }
    
    /// Set the site as the default for this client.
    ///
    /// # Arguments
    ///
    /// * `site` - The site to set as default.
    ///
    /// # Returns
    ///
    /// The updated client.
    pub fn set_as_default(&self, site: &Site) -> UnifiClient {
        let mut new_client = self.client.clone();
        new_client.config.site = site.name.clone();
        new_client
    }
    
    /// Get site statistics.
    ///
    /// # Returns
    ///
    /// Statistics for the current site.
    pub async fn stats(&self) -> UnifiResult<SiteStats> {
        let mut client = self.client.clone();
        
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/stat/health", site);
        
        let stats: Vec<SiteStats> = client.request(Method::GET, &endpoint, None::<()>).await?;
        
        stats.into_iter()
            .next()
            .ok_or_else(|| UnifiError::ApiError("No site statistics available".to_string()))
    }
}