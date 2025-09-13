use http::Method;
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
        let response: Value = client
            .request_json(Method::POST, &endpoint, Some(payload))
            .await?;

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
            .request_json(Method::POST, &endpoint, Some(auth_payload))
            .await?;

        // Get the list of guests
        let endpoint = format!("/api/s/{}/stat/guest", site);
        let response: Value = client
            .request_json(Method::GET, &endpoint, None::<()>)
            .await?;

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
            .request_json(Method::POST, &endpoint, Some(auth_payload))
            .await?;

        // Now unauthorize the guest
        let unauth_payload = serde_json::json!({
            "cmd": "unauthorize-guest",
            "mac": test_mac,
        });

        // Make raw API call
        let response = client
            .request_json(Method::POST, &endpoint, Some(unauth_payload))
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
            let result = client
                .request_json(Method::POST, &endpoint, Some(payload))
                .await;

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

        println!("\nTesting MAC address format acceptance...");

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
            let result = client
                .request_json(Method::POST, &endpoint, Some(payload))
                .await;

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
            let result = client
                .request_json(Method::POST, &endpoint, Some(payload))
                .await;

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

    async fn validate_bytes_parameter(&self) -> UniFiResult<()> {
        let client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        println!("\nTesting bytes parameter for guest authorization...");

        // Test various byte limits
        let test_values = [
            (-1000, "negative bytes"),
            (1, "1 MB"),
            (10, "10 MB"),
            (100, "100 MB"),
            (500, "500 MB"),
            (1_000, "1 GB"),
            (5_000, "5 GB"),
            (10_000, "10 GB"),
            (50_000, "50 GB"),
            (100_000, "100 GB"),
            (0, "zero bytes"),
        ];

        for (bytes, description) in &test_values {
            let test_mac = random_mac();

            // Create authorization request with bytes instead of minutes
            let payload = serde_json::json!({
                "cmd": "authorize-guest",
                "mac": test_mac,
                "bytes": bytes,
            });

            // Make raw API call and check if it succeeds
            let result = client
                .request_json(Method::POST, &endpoint, Some(payload))
                .await;

            match result {
                Ok(response) => {
                    if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
                        // Check if the response indicates success
                        if auth["qos_usage_quota"].is_number() {
                            // Ensure QoS overwrite is enabled
                            if let Some(qos_overwritten) = auth["qos_overwrite"].as_bool() {
                                if qos_overwritten == false {
                                    println!(
                                        "⚠️ Bytes = {}: {} accepted but QoS overwrite was disabled: {}",
                                        bytes, description, qos_overwritten
                                    );
                                }
                            } else {
                                println!(
                                    "⚠️ Bytes = {}: {} accepted but no qos_overwrite in response",
                                    bytes, description
                                );
                                continue;
                            };
                            // Ensure bytes limit was accepted
                            if let Some(usage_quota) = auth["qos_usage_quota"].as_i64() {
                                let expected_bytes = if *bytes < 0 { 0 } else { *bytes };
                                if usage_quota == expected_bytes {
                                    println!(
                                        "✅ Bytes = {}: {} accepted with correct limit",
                                        bytes, description
                                    );
                                } else {
                                    println!(
                                        "⚠️ Bytes = {}: {} accepted but with adjusted/invalid limit: {}",
                                        bytes, description, usage_quota
                                    );
                                }
                            } else {
                                println!(
                                    "⚠️ Bytes = {}: {} accepted but no quota in response",
                                    bytes, description
                                );
                            }
                        } else {
                            println!(
                                "❌ Bytes = {}: {} failed - unexpected response structure",
                                bytes, description
                            );
                        }
                    } else {
                        println!(
                            "❌ Bytes = {}: {} failed - empty or invalid response",
                            bytes, description
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "❌ Bytes = {}: {} rejected with error: {}",
                        bytes, description, e
                    );
                }
            }

            // Add a small delay between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Test combining bytes with minutes
        println!("\nTesting combination of bytes and minutes parameters...");

        let test_mac = random_mac();
        let payload = serde_json::json!({
            "cmd": "authorize-guest",
            "mac": test_mac,
            "minutes": 60,
            "bytes": 100, // 100 MB
        });

        let result = client
            .request_json(Method::POST, &endpoint, Some(payload))
            .await;

        match result {
            Ok(response) => {
                if let Some(auth) = response.as_array().and_then(|arr| arr.first()) {
                    if auth["_id"].is_string() {
                        let has_time_limit = auth["end"].is_u64() && auth["start"].is_u64();
                        let has_byte_limit = auth["qos_usage_quota"].is_number();

                        if has_time_limit && has_byte_limit {
                            println!("✅ Combined bytes + minutes: Both limits applied");
                        } else if has_time_limit {
                            println!("⚠️ Combined bytes + minutes: Only time limit applied");
                        } else if has_byte_limit {
                            println!("⚠️ Combined bytes + minutes: Only byte limit applied");
                        } else {
                            println!("❌ Combined bytes + minutes: Neither limit detected");
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Combined bytes + minutes rejected: {}", e);
            }
        }

        println!("Bytes parameter testing complete");
        Ok(())
    }

    async fn validate_speed_limits(&self) -> UniFiResult<()> {
        let client = self.client.clone();
        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/stamgr", site);

        println!("\nTesting speed limits (Kbps) for guest authorization...");

        // Helper to perform an authorization and return first array entry
        async fn auth_with(
            client: &UniFiClient,
            endpoint: &str,
            up: Option<i64>,
            down: Option<i64>,
            minutes: i64,
        ) -> UniFiResult<Value> {
            let mac = random_mac();
            let mut payload = serde_json::json!({
                "cmd": "authorize-guest",
                "mac": mac,
                "minutes": minutes,
            });

            if let Some(u) = up {
                payload["up"] = serde_json::json!(u);
            }
            if let Some(d) = down {
                payload["down"] = serde_json::json!(d);
            }

            let response: Value = client
                .request_json(Method::POST, endpoint, Some(payload))
                .await?;

            if let Some(first) = response.as_array().and_then(|a| a.first()).cloned() {
                Ok(first)
            } else {
                Ok(response)
            }
        }

        // Verify a response contains qos_overwrite true and optional exact matches
        fn check_qos(
            context: &str,
            auth: &Value,
            expect_up: Option<i64>,
            expect_down: Option<i64>,
        ) {
            let mut passed = true;

            // qos_overwrite must be present and true
            match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                Some(true) => { /* ok */ }
                Some(false) => {
                    println!("❌ {}: qos_overwrite present but false", context);
                    passed = false;
                }
                None => {
                    println!("❌ {}: qos_overwrite missing or not a boolean", context);
                    passed = false;
                }
            }

            // If up was requested, ensure qos_rate_max_up matches exactly
            if let Some(expected) = expect_up {
                match auth.get("qos_rate_max_up").and_then(|v| v.as_i64()) {
                    Some(v) if v == expected => { /* ok */ }
                    Some(v) => {
                        println!(
                            "❌ {}: qos_rate_max_up mismatch. expected={}, got={}",
                            context, expected, v
                        );
                        passed = false;
                    }
                    None => {
                        println!("❌ {}: qos_rate_max_up missing or not a number", context);
                        passed = false;
                    }
                }
            }

            // If down was requested, ensure qos_rate_max_down matches exactly
            if let Some(expected) = expect_down {
                match auth.get("qos_rate_max_down").and_then(|v| v.as_i64()) {
                    Some(v) if v == expected => { /* ok */ }
                    Some(v) => {
                        println!(
                            "❌ {}: qos_rate_max_down mismatch. expected={}, got={}",
                            context, expected, v
                        );
                        passed = false;
                    }
                    None => {
                        println!("❌ {}: qos_rate_max_down missing or not a number", context);
                        passed = false;
                    }
                }
            }

            if passed {
                println!("✅ {}: speed limit settings applied as expected", context);
            }
        }

        // Independent tests
        // up only
        match auth_with(&client, &endpoint, Some(500), None, 60).await {
            Ok(auth) => check_qos("up only (500 Kbps)", &auth, Some(500), None),
            Err(e) => println!("❌ up only request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // down only
        match auth_with(&client, &endpoint, None, Some(700), 60).await {
            Ok(auth) => check_qos("down only (700 Kbps)", &auth, None, Some(700)),
            Err(e) => println!("❌ down only request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // both up and down
        match auth_with(&client, &endpoint, Some(400), Some(800), 60).await {
            Ok(auth) => check_qos(
                "both up=400 Kbps, down=800 Kbps",
                &auth,
                Some(400),
                Some(800),
            ),
            Err(e) => println!("❌ up+down request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Document behavior for zero values
        println!("\nObserving behavior for zero values (treated by controller as-is or clamped). Units are Kbps.");
        match auth_with(&client, &endpoint, Some(0), None, 30).await {
            Ok(auth) => {
                let ctx = "up only (0 Kbps)";
                // We do not assert pass/fail here beyond qos_overwrite presence; we report observed
                // values.
                if let Some(qos) = auth.get("qos_rate_max_up").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: controller returned qos_rate_max_up={}", ctx, qos);
                } else {
                    println!("ℹ️ {}: controller did not return qos_rate_max_up", ctx);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ up=0 request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        match auth_with(&client, &endpoint, None, Some(0), 30).await {
            Ok(auth) => {
                let ctx = "down only (0 Kbps)";
                if let Some(qos) = auth.get("qos_rate_max_down").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: controller returned qos_rate_max_down={}", ctx, qos);
                } else {
                    println!("ℹ️ {}: controller did not return qos_rate_max_down", ctx);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ down=0 request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Document behavior for negative values
        println!("\nObserving behavior for negative values (controller may clamp or reject). Units are Kbps.");
        match auth_with(&client, &endpoint, Some(-500), None, 30).await {
            Ok(auth) => {
                let ctx = "up only (-500 Kbps)";
                if let Some(qos) = auth.get("qos_rate_max_up").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: controller returned qos_rate_max_up={}", ctx, qos);
                } else {
                    println!("ℹ️ {}: controller did not return qos_rate_max_up", ctx);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ up=-500 request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        match auth_with(&client, &endpoint, None, Some(-500), 30).await {
            Ok(auth) => {
                let ctx = "down only (-500 Kbps)";
                if let Some(qos) = auth.get("qos_rate_max_down").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: controller returned qos_rate_max_down={}", ctx, qos);
                } else {
                    println!("ℹ️ {}: controller did not return qos_rate_max_down", ctx);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ down=-500 request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Combined zero/negative for completeness
        match auth_with(&client, &endpoint, Some(0), Some(0), 30).await {
            Ok(auth) => {
                let ctx = "both up=0, down=0";
                if let Some(v) = auth.get("qos_rate_max_up").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: qos_rate_max_up={}", ctx, v);
                }
                if let Some(v) = auth.get("qos_rate_max_down").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: qos_rate_max_down={}", ctx, v);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ up=0,down=0 request failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        match auth_with(&client, &endpoint, Some(-250), Some(-250), 30).await {
            Ok(auth) => {
                let ctx = "both up=-250, down=-250";
                if let Some(v) = auth.get("qos_rate_max_up").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: qos_rate_max_up={}", ctx, v);
                }
                if let Some(v) = auth.get("qos_rate_max_down").and_then(|v| v.as_i64()) {
                    println!("ℹ️ {}: qos_rate_max_down={}", ctx, v);
                }
                match auth.get("qos_overwrite").and_then(|v| v.as_bool()) {
                    Some(true) => println!("✅ {}: qos_overwrite=true", ctx),
                    Some(false) => println!("⚠️ {}: qos_overwrite=false", ctx),
                    None => println!("⚠️ {}: qos_overwrite missing", ctx),
                }
            }
            Err(e) => println!("❌ up=-250,down=-250 request failed: {}", e),
        }

        println!("Speed limit testing complete");
        Ok(())
    }

    pub async fn run_all_validations(&self) -> UniFiResult<()> {
        println!("Running guest validator...");
        self.validate_authorize_simple_duration().await?;
        self.validate_list_guests().await?;
        self.validate_unauthorize().await?;
        self.validate_minutes_parameter_range().await?;
        self.validate_mac_address_formats().await?;
        self.validate_bytes_parameter().await?;
        self.validate_speed_limits().await?;
        Ok(())
    }
}
