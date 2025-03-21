use rand::{Rng, rng};

/// Generate a random MAC address in the format "00:11:22:33:44:55"
pub fn random_mac() -> String {
    let mut rng = rng();
    let mut mac = String::with_capacity(17);
    
    for i in 0..6 {
        if i > 0 {
            mac.push(':');
        }
        // Generate two random hex digits
        let byte: u8 = rng.random();
        mac.push_str(&format!("{:02x}", byte));
    }
    
    mac
}