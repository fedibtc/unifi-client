use rand::{thread_rng, Rng};

/// Generate a random MAC address in the format "00:11:22:33:44:55"
pub fn random_mac() -> String {
    let mut rng = thread_rng();
    let mut mac = String::with_capacity(17);
    
    for i in 0..6 {
        if i > 0 {
            mac.push(':');
        }
        // Generate two random hex digits
        let byte: u8 = rng.gen();
        mac.push_str(&format!("{:02x}", byte));
    }
    
    mac
}