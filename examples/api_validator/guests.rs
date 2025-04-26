use serde_json::Value;
use unifi_client::{UniFiClient, UniFiResult};

use crate::utils::random_mac;

pub struct GuestsValidator {
    client: UniFiClient,
}

impl GuestsValidator {
    pub fn new(client: UniFiClient) -> Self {
        Self { client }
    }

    async fn validate_authorize_simple_duration(&self) -> UniFiResult<()> {
        let client = self.client.clone();

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

    async fn validate_list_guests(&self) -> UniFiResult<()> {
        let client = self.client.clone();
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
                println!(
                    "✅ List authorized guests test passed for {} guest entries",
                    guests.len()
                );
            }
        } else {
            println!("❌ Invalid response format: expected array");
        }

        Ok(())
    }

    async fn validate_unauthorize(&self) -> UniFiResult<()> {
        let client = self.client.clone();

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

    async fn validate_minutes_parameter_range(&self) -> UniFiResult<()> {
        let client = self.client.clone();
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
                            if let (Some(start), Some(end)) =
                                (auth["start"].as_u64(), auth["end"].as_u64())
                            {
                                let actual_minutes = (end - start) / 60;
                                let expected_minutes =
                                    if *minutes < 0 { 0 } else { *minutes as u64 };

                                if actual_minutes == expected_minutes {
                                    println!(
                                        "✅ Minutes = {}: {} accepted with correct duration",
                                        minutes, description
                                    );
                                } else {
                                    println!("⚠️ Minutes = {}: {} accepted but with adjusted duration: {}", 
                                             minutes, description, actual_minutes);
                                }
                            } else {
                                println!(
                                    "⚠️ Minutes = {}: {} accepted but couldn't validate duration",
                                    minutes, description
                                );
                            }
                        } else {
                            println!(
                                "❌ Minutes = {}: {} failed - unexpected response structure",
                                minutes, description
                            );
                        }
                    } else {
                        println!(
                            "❌ Minutes = {}: {} failed - empty or invalid response",
                            minutes, description
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "❌ Minutes = {}: {} rejected with error: {}",
                        minutes, description, e
                    );
                }
            }

            // Add a small delay between requests to avoid overwhelming the controller
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        println!("Minutes parameter range testing complete");
        Ok(())
    }

    async fn validate_mac_address_formats(&self) -> UniFiResult<()> {
        let client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        println!("Testing MAC address format acceptance...");

        // Generate a random MAC in standard format
        let standard_mac = random_mac(); // This is colon-separated: 00:11:22:33:44:55

        // Create variations of the same MAC
        let without_colons = standard_mac.replace(":", ""); // 001122334455
        let with_hyphens = standard_mac.replace(":", "-"); // 00-11-22-33-44-55
        let uppercase = standard_mac.to_uppercase(); // 00:11:22:33:44:55
        let mixed_case = standard_mac
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect::<String>(); // 0A:1B:2C:3D:4E:5F

        // Test different formats
        let test_formats = [
            (
                standard_mac.clone(),
                "Standard colon-separated (00:11:22:33:44:55)",
            ),
            (without_colons, "No separators (001122334455)"),
            (with_hyphens, "Hyphen-separated (00-11-22-33-44-55)"),
            (uppercase, "Uppercase (00:11:22:33:44:55)"),
            (mixed_case, "Mixed case (0A:1B:2C:3D:4E:5F)"),
        ];

        for (mac, description) in &test_formats {
            // Create authorization request
            let payload = serde_json::json!({
                "cmd": "authorize-guest",
                "mac": mac,
                "minutes": 5, // Short duration to avoid cluttering the system
            });

            // Make raw API call and check if it succeeds
            let result = client.raw_request("POST", &endpoint, Some(payload)).await;

            match result {
                Ok(response) => {
                    if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
                        // Check if the response indicates success and includes a MAC
                        if let Some(returned_mac) = auth["mac"].as_str() {
                            // Check if the returned MAC is normalized to a particular format
                            if returned_mac == &standard_mac {
                                println!(
                                    "✅ Format accepted: {} - Controller returned standard format",
                                    description
                                );
                            } else {
                                println!(
                                    "✅ Format accepted: {} - Controller normalized to: {}",
                                    description, returned_mac
                                );
                            }
                        } else {
                            println!(
                                "⚠️ Format potentially accepted: {} - But no MAC in response",
                                description
                            );
                        }
                    } else {
                        println!(
                            "❌ Format rejected: {} - Empty or invalid response",
                            description
                        );
                    }
                }
                Err(e) => {
                    println!("❌ Format rejected: {} - Error: {}", description, e);
                }
            }

            // Add a small delay between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Additional edge cases worth testing
        let edge_cases = [
            ("0:1:2:3:4:5", "Short single digits"),
            ("0:11:22:33:44:55", "Missing leading zero"),
            ("000:111:222:333:444:555", "Extra digits"),
            ("00:11:22:33:44", "Incomplete (5 octets)"),
            ("00:11:22:33:44:55:66", "Too long (7 octets)"),
            ("GG:HH:II:JJ:KK:LL", "Invalid hex characters"),
        ];

        println!("\nTesting edge cases...");
        for (mac, description) in &edge_cases {
            // Create authorization request
            let payload = serde_json::json!({
                "cmd": "authorize-guest",
                "mac": mac,
                "minutes": 5,
            });

            // Make raw API call and check if it succeeds
            let result = client.raw_request("POST", &endpoint, Some(payload)).await;

            match result {
                Ok(response) => {
                    if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
                        // Check if the response indicates success and includes a MAC
                        if let Some(returned_mac) = auth["mac"].as_str() {
                            println!(
                                "✅ Edge case accepted: {} - Controller normalized to: {}",
                                description, returned_mac
                            );
                        } else {
                            println!(
                                "⚠️ Edge case potentially accepted: {} - But no MAC in response",
                                description
                            );
                        }
                    } else {
                        println!(
                            "❌ Edge case rejected: {} - Empty or invalid response",
                            description
                        );
                    }
                }
                Err(e) => {
                    println!("❌ Edge case rejected: {} - Error: {}", description, e);
                }
            }

            // Add a small delay between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        println!("MAC address format testing complete");
        Ok(())
    }

    pub async fn run_all_validations(&self) -> UniFiResult<()> {
        println!("Running guest validator...");
        self.validate_authorize_simple_duration().await?;
        self.validate_list_guests().await?;
        self.validate_unauthorize().await?;
        self.validate_minutes_parameter_range().await?;
        self.validate_mac_address_formats().await?;
        Ok(())
    }
}
