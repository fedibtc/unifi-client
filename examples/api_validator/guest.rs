use serde_json::Value;

use crate::utils::random_mac;
use unifi_client::{UnifiClient, UnifiResult};

pub struct GuestValidator {
    client: UnifiClient,
}

impl GuestValidator {
    pub fn new(client: UnifiClient) -> Self {
        Self { client }
    }

    async fn validate_authorize_simple_duration(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();

        // Test MAC address and duration
        let test_mac = random_mac();
        let test_duration = 30; // 30 minutes

        // Create authorization request
        let payload = serde_json::json!({
            "cmd": "authorize-guest",
            "mac": test_mac,
            "minutes": test_duration,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        // Make raw API call
        let response: Value = client.raw_request("POST", &endpoint, Some(payload)).await?;

        // Validate response structure and values
        if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
            let mut passed = true;

            // Check all required fields exist and have correct types
            if !auth["_id"].is_string() {
                println!("❌ Missing or invalid _id field");
                passed = false;
            }

            if auth["authorized_by"].as_str() != Some("api") {
                println!("❌ Unexpected authorized_by value");
                passed = false;
            }

            if !auth["end"].is_u64() {
                println!("❌ Missing or invalid end timestamp");
                passed = false;
            }

            if auth["mac"].as_str() != Some(&test_mac) {
                println!("❌ MAC address mismatch");
                passed = false;
            }

            if !auth["site_id"].is_string() {
                println!("❌ Missing or invalid site_id");
                passed = false;
            }

            if !auth["start"].is_u64() {
                println!("❌ Missing or invalid start timestamp");
                passed = false;
            }

            // Validate duration by checking end - start
            if let (Some(start), Some(end)) = (auth["start"].as_u64(), auth["end"].as_u64()) {
                let duration_minutes = (end - start) / 60;
                if duration_minutes != test_duration as u64 {
                    println!(
                        "❌ Duration mismatch: expected {} minutes, got {}",
                        test_duration, duration_minutes
                    );
                    passed = false;
                }
            } else {
                println!("❌ Could not validate duration: missing timestamps");
                passed = false;
            }

            if passed {
                println!("✅ Guest authorization test passed with all expected values");
            }
        } else {
            println!("❌ Guest authorization test failed: unexpected response format");
        }

        Ok(())
    }

    async fn validate_list_guests(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        let site = self.client.site();

        // First authorize a guest so we can ensure there are guests to list
        let test_mac = random_mac();

        // Create authorization request
        let auth_payload = serde_json::json!({
            "cmd": "authorize-guest",
            "mac": test_mac,
            "minutes": 30,
        });

        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        // Authorize the guest first
        let _: Value = client
            .raw_request("POST", &endpoint, Some(auth_payload))
            .await?;

        // Get the list of guests
        let endpoint = format!("/api/s/{}/stat/guest", site);
        let response: Value = client.raw_request("GET", &endpoint, None::<()>).await?;

        // Validate response is an array
        if let Some(guests) = response.as_array() {
            if guests.is_empty() {
                println!("✅ Empty guest list is valid");
                return Ok(());
            }

            let mut valid = true;
            for (i, guest) in guests.iter().enumerate() {
                // Check required fields exist and have correct types
                if !guest["_id"].is_string() {
                    println!("❌ Guest {} missing or invalid _id field", i);
                    valid = false;
                }
                if !guest["authorized_by"].is_string() {
                    println!("❌ Guest {} missing or invalid authorized_by field", i);
                    valid = false;
                }
                if !guest["end"].is_u64() {
                    println!("❌ Guest {} missing or invalid end timestamp", i);
                    valid = false;
                }
                if !guest["expired"].is_boolean() {
                    println!("❌ Guest {} missing or invalid expired field", i);
                    valid = false;
                }
                if !guest["mac"].is_string() {
                    println!("❌ Guest {} missing or invalid mac field", i);
                    valid = false;
                }
                if !guest["site_id"].is_string() {
                    println!("❌ Guest {} missing or invalid site_id field", i);
                    valid = false;
                }
                if !guest["start"].is_u64() {
                    println!("❌ Guest {} missing or invalid start timestamp", i);
                    valid = false;
                }
            }

            if valid {
                println!("✅ List authorized guests test passed for {} guest entries", guests.len());
            }
        } else {
            println!("❌ Invalid response format: expected array");
        }

        Ok(())
    }

    async fn validate_unauthorize(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();

        // First authorize a guest so we can then unauthorize them
        let test_mac = random_mac();

        // Create authorization request
        let auth_payload = serde_json::json!({
            "cmd": "authorize-guest",
            "mac": test_mac,
            "minutes": 30,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        // Authorize the guest first
        let _: Value = client
            .raw_request("POST", &endpoint, Some(auth_payload))
            .await?;

        // Now unauthorize the guest
        let unauth_payload = serde_json::json!({
            "cmd": "unauthorize-guest",
            "mac": test_mac,
        });

        // Make raw API call
        let response = client
            .raw_request("POST", &endpoint, Some(unauth_payload))
            .await?;

        // Validate response is an empty array
        match response.as_array() {
            Some(arr) if arr.is_empty() => {
                println!("✅ Unauthorize guest test passed with expected response")
            }
            Some(_) => println!("❌ Expected empty array response, got non-empty array"),
            None => println!("❌ Expected array response, got different type"),
        }

        Ok(())
    }

    async fn validate_minutes_parameter_range(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);
        
        // Test values to try
        let test_values = [
            (-1, "negative value"),
            (0, "zero"),
            (1, "minimum positive"),
            (60, "1 hour"),
            (1440, "1 day"),
            (10080, "1 week"),
            (43200, "30 days"),
            (525600, "1 year"),
            (1051200, "2 years"),
        ];
        
        for (minutes, description) in &test_values {
            // Generate a random MAC for each test
            let test_mac = random_mac();
            
            // Create authorization request
            let payload = serde_json::json!({
                "cmd": "authorize-guest",
                "mac": test_mac,
                "minutes": minutes,
            });
            
            // Make raw API call and check if it succeeds
            let result = client.raw_request("POST", &endpoint, Some(payload)).await;
            
            match result {
                Ok(response) => {
                    if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
                        // Check if the response indicates success
                        if auth["_id"].is_string() {
                            // Validate that the duration matches what was requested
                            if let (Some(start), Some(end)) = (auth["start"].as_u64(), auth["end"].as_u64()) {
                                let actual_minutes = (end - start) / 60;
                                let expected_minutes = if *minutes < 0 { 0 } else { *minutes as u64 };
                                
                                if actual_minutes == expected_minutes {
                                    println!("✅ Minutes = {}: {} accepted with correct duration", minutes, description);
                                } else {
                                    println!("⚠️ Minutes = {}: {} accepted but with adjusted duration: {}", 
                                             minutes, description, actual_minutes);
                                }
                            } else {
                                println!("⚠️ Minutes = {}: {} accepted but couldn't validate duration", 
                                         minutes, description);
                            }
                        } else {
                            println!("❌ Minutes = {}: {} failed - unexpected response structure", 
                                     minutes, description);
                        }
                    } else {
                        println!("❌ Minutes = {}: {} failed - empty or invalid response", 
                                 minutes, description);
                    }
                },
                Err(e) => {
                    println!("❌ Minutes = {}: {} rejected with error: {}", minutes, description, e);
                }
            }
            
            // Add a small delay between requests to avoid overwhelming the controller
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        println!("Minutes parameter range testing complete");
        Ok(())
    }

    pub async fn run_all_validations(&self) -> UnifiResult<()> {
        println!("Running guest validator...");
        self.validate_authorize_simple_duration().await?;
        self.validate_list_guests().await?;
        self.validate_unauthorize().await?;
        self.validate_minutes_parameter_range().await?;
        Ok(())
    }
}
