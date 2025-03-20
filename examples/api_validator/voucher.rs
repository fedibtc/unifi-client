use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use unifi_client::{UnifiClient, UnifiError, UnifiResult, VoucherExpireUnit};

pub struct VoucherValidator {
    client: UnifiClient,
}

impl VoucherValidator {
    pub fn new(client: UnifiClient) -> Self {
        Self { client }
    }

    async fn validate_simple_duration(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        // Create a voucher with 30 minute duration
        let create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": 1,
            "expire": 30,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        // Make raw API call instead of using built-in API
        let create_response = client.raw_request("POST", &endpoint, Some(create_data)).await?;
        
        // Extract and validate create_time from response
        let create_time = create_response
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|obj| obj.get("create_time"))
            .and_then(|time| time.as_i64())
            .ok_or_else(|| UnifiError::ApiError("Invalid create-voucher response format".into()))?;

        // Validate the created voucher
        let get_endpoint = format!("/api/s/{}/stat/voucher", site);
        let get_data = serde_json::json!({
            "create_time": create_time
        });
        let vouchers: Value = client.raw_request("GET", &get_endpoint, Some(get_data)).await?;
        
        // Validate duration is 30
        if let Some(voucher) = vouchers.as_array().and_then(|v| v.first()) {
            if voucher["duration"].as_i64() == Some(30) {
                println!("✅ Simple 'expire' duration test passed");
            } else {
                println!("❌ Simple 'expire' duration test failed: expected duration 30, got {:?}", 
                    voucher["duration"]);
            }
        }
        
        Ok(())
    }

    async fn validate_minutes_unit_duration(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        // Create a voucher with 5 minute duration
        let create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": 1,
            "expire_number": 5,
            "expire_unit": VoucherExpireUnit::Minutes,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        let _create_response: Value = client.raw_request("POST", &endpoint, Some(create_data)).await?;
        
        // Get creation timestamp
        let create_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Validate the created voucher
        let get_endpoint = format!("/api/s/{}/stat/voucher", site);
        let get_data = serde_json::json!({
            "create_time": create_time
        });
        
        let vouchers: Value = client.raw_request("GET", &get_endpoint, Some(get_data)).await?;
        
        // Validate duration is 300 (5 minutes * 60 seconds)
        if let Some(voucher) = vouchers.as_array().and_then(|v| v.first()) {
            if voucher["duration"].as_i64() == Some(300) {
                println!("✅ Minute unit duration test passed");
            } else {
                println!("❌ Minute unit duration test failed: expected duration 300, got {:?}", 
                    voucher["duration"]);
            }
        }
        
        Ok(())
    }

    async fn validate_hours_unit_duration(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        // Create a voucher with 5 hour duration
        let create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": 1,
            "expire_number": 5,
            "expire_unit": VoucherExpireUnit::Hours,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        let _create_response: Value = client.raw_request("POST", &endpoint, Some(create_data)).await?;
        
        // Get creation timestamp
        let create_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Validate the created voucher
        let get_endpoint = format!("/api/s/{}/stat/voucher", site);
        let get_data = serde_json::json!({
            "create_time": create_time
        });
        
        let vouchers: Value = client.raw_request("GET", &get_endpoint, Some(get_data)).await?;
        
        // Validate duration is 18000 (5 hours * 60 minutes * 60 seconds)
        if let Some(voucher) = vouchers.as_array().and_then(|v| v.first()) {
            if voucher["duration"].as_i64() == Some(18000) {
                println!("✅ Hour unit duration test passed");
            } else {
                println!("❌ Hour unit duration test failed: expected duration 300, got {:?}", 
                    voucher["duration"]);
            }
        }
        
        Ok(())
    }

    async fn validate_voucher_note(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        let test_note = "Test note for validation";
        
        // Create a voucher with a note
        let create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": 1,
            "expire": 30,
            "note": test_note,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        // Make raw API call and extract create_time
        let create_response = client.raw_request("POST", &endpoint, Some(create_data)).await?;
        let create_time = create_response
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|obj| obj.get("create_time"))
            .and_then(|time| time.as_i64())
            .ok_or_else(|| UnifiError::ApiError("Invalid create-voucher response format".into()))?;

        // Validate the created voucher
        let get_endpoint = format!("/api/s/{}/stat/voucher", site);
        let get_data = serde_json::json!({
            "create_time": create_time
        });
        
        let vouchers: Value = client.raw_request("GET", &get_endpoint, Some(get_data)).await?;
        
        // Validate note matches
        if let Some(voucher) = vouchers.as_array().and_then(|v| v.first()) {
            if voucher["note"].as_str() == Some(test_note) {
                println!("✅ Voucher note test passed");
            } else {
                println!("❌ Voucher note test failed: expected note '{}', got {:?}", 
                    test_note, voucher["note"]);
            }
        }
        
        Ok(())
    }

    async fn validate_data_transmit_limit(&self) -> UnifiResult<()> {
        let mut client = self.client.clone();
        
        let transfer_limit = 1000; // Test with 1GB quota
        
        // Create a voucher with data quota
        let create_data = serde_json::json!({
            "cmd": "create-voucher",
            "n": 1,
            "expire": 30,
            "bytes": transfer_limit,
        });

        let site = self.client.site();
        let endpoint = format!("/api/s/{}/cmd/hotspot", site);
        
        // Make raw API call and extract create_time
        let create_response = client.raw_request("POST", &endpoint, Some(create_data)).await?;
        let create_time = create_response
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|obj| obj.get("create_time"))
            .and_then(|time| time.as_i64())
            .ok_or_else(|| UnifiError::ApiError("Invalid create-voucher response format".into()))?;

        // Validate the created voucher
        let get_endpoint = format!("/api/s/{}/stat/voucher", site);
        let get_data = serde_json::json!({
            "create_time": create_time
        });
        
        let vouchers: Value = client.raw_request("GET", &get_endpoint, Some(get_data)).await?;
        
        // Validate quota matches
        if let Some(voucher) = vouchers.as_array().and_then(|v| v.first()) {
            if let Some(quota) = voucher["qos_usage_quota"].as_u64() {
                if quota == transfer_limit as u64 {
                    println!("✅ Data quota test passed");
                } else {
                    println!("❌ Data quota test failed: expected {} MB, got {} MB", 
                        transfer_limit, quota);
                }
            } else {
                println!("❌ Data quota test failed: quota field not found or invalid type");
            }
        }
        
        Ok(())
    }

    pub async fn run_all_validations(&mut self) -> UnifiResult<()> {
        println!("Running voucher validator...");
        self.validate_simple_duration().await?;
        self.validate_minutes_unit_duration().await?;
        self.validate_hours_unit_duration().await?;
        self.validate_voucher_note().await?;
        self.validate_data_transmit_limit().await?;
        Ok(())
    }
}