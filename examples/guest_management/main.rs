use std::env;
use std::error::Error;
use std::io::{self, Write};

use chrono::{DateTime, Utc};
use env_logger;
use unifi_client::models::guests::GuestEntry;
use unifi_client::UniFiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enable logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Get configuration from environment variables
    let controller = env::var("UNIFI_CONTROLLER")
        .unwrap_or_else(|_| "https://unifi.example.com:8443".to_string());
    let username = env::var("UNIFI_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let site = env::var("UNIFI_SITE").unwrap_or_else(|_| "default".to_string());
    let verify_ssl = env::var("UNIFI_VERIFY_SSL")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    println!("UniFi Guest Management Example");
    println!("==============================");
    println!("Controller: {}", controller);
    println!("Site: {}", site);

    // Create and authenticate the UniFi client using the builder
    let client = UniFiClient::builder()
        .controller_url(&controller)
        .username(&username)
        .password_from_env("UNIFI_PASSWORD")
        .site(&site)
        .verify_ssl(verify_ssl)
        .build()
        .await?;
    println!("✅ Authentication successful!");

    // Get a reference to the guest API handler
    let guest_handler = client.guests(); // Get the handler

    // Display menu
    loop {
        println!("\nGuest Management Options:");
        println!("1. List Active Guests");
        println!("2. List Expired Guests");
        println!("3. Authorize Guest");
        println!("4. Unauthorize Guest");
        println!("5. Unauthorize All Guests");
        println!("6. Exit");
        print!("\nSelect an option (1-6): ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        match choice.trim() {
            "1" => {
                println!("\nFetching active guests...");
                let guests = guest_handler.list().send().await?;
                let active_guests: Vec<_> = guests
                    .into_iter()
                    .filter(|guest| !guest.is_expired())
                    .collect();

                if active_guests.is_empty() {
                    println!("No active guests found.");
                } else {
                    println!("\nFound {} active guests:", active_guests.len());
                    println!(
                        "{:<26} {:<20} {:<12} {:<30}",
                        "ID", "MAC", "Status", "Expires At (UTC)"
                    );
                    println!("{}", "-".repeat(80));

                    for guest in active_guests {
                        let expires_timestamp = guest.expires_at();
                        let expires_dt =
                            DateTime::<Utc>::from_timestamp(expires_timestamp as i64, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                .unwrap_or_else(|| "Invalid timestamp".to_string());
                        println!(
                            "{:<26} {:<20} {:<12} {:<30}",
                            guest.id(),
                            guest.mac(),
                            "Active",
                            expires_dt,
                        );
                    }
                }
            }
            "2" => {
                println!("\nFetching expired guests...");
                // Use the ListGuestsBuilder
                let guests = guest_handler.list().send().await?;
                let expired_guests: Vec<_> = guests
                    .into_iter()
                    .filter(|guest| guest.is_expired())
                    .collect();

                if expired_guests.is_empty() {
                    println!("No expired guests found.");
                } else {
                    println!("\nFound {} expired guests:", expired_guests.len());
                    println!("{:<26} {:<20} {:<25}", "ID", "MAC", "Unauthorized By");
                    println!("{}", "-".repeat(80));

                    for guest in expired_guests {
                        // Get all needed fields before the match
                        let id = guest.id();
                        let mac = guest.mac();
                        let unauthorized_by = match &guest {
                            GuestEntry::Inactive {
                                unauthorized_by, ..
                            } => unauthorized_by.clone().unwrap_or_else(|| "".to_string()),
                            _ => "".to_string(),
                        };
                        println!("{:<26} {:<20} {:<25}", id, mac, unauthorized_by,);
                    }
                }
            }
            "3" => {
                println!("\nAuthorize New Guest");
                print!("Enter MAC address (e.g., 00:11:22:33:44:55): ");
                io::stdout().flush().unwrap();
                let mut mac = String::new();
                io::stdin().read_line(&mut mac)?;
                let mac = mac.trim().to_string();

                print!("Duration in minutes (e.g., 1440 for 1 day): ");
                io::stdout().flush().unwrap();
                let mut duration_str = String::new();
                io::stdin().read_line(&mut duration_str)?;
                let duration: u32 = duration_str.trim().parse()?;

                println!("\nAuthorizing guest...");
                // Use the AuthorizeGuestBuilder
                let guest = guest_handler
                    .authorize(&mac)
                    .duration_minutes(duration)
                    .send() // Send the request via the builder
                    .await?;
                println!("✅ Successfully authorized guest: {}", guest.mac());
            }
            "4" => {
                println!("\nUnauthorize a Guest");
                // Use the ListGuestsBuilder
                let guests = guest_handler.list().send().await?;

                if guests.is_empty() {
                    println!("No guests available to unauthorize.");
                    continue;
                }

                println!("\nAvailable guests:");
                println!("{:<5} {:<20} {:<12}", "Num", "MAC", "Status");
                println!("{}", "-".repeat(40));

                for (i, guest) in guests.iter().enumerate() {
                    let status = if guest.is_expired() {
                        "Expired"
                    } else if guest.was_unauthorized() {
                        "Unauthorized"
                    } else {
                        "Active"
                    };

                    println!("{:<5} {:<20} {:<12}", i + 1, guest.mac(), status);
                }

                print!("\nEnter guest number to unauthorize (or 0 to cancel): ");
                io::stdout().flush().unwrap();
                let mut selection = String::new();
                io::stdin().read_line(&mut selection)?;
                let selection: usize = selection.trim().parse()?;

                if selection == 0 || selection > guests.len() {
                    println!("Operation cancelled or invalid selection.");
                    continue;
                }

                let selected_guest = &guests[selection - 1];

                print!(
                    "Are you sure you want to unauthorize guest {} (y/n)? ",
                    selected_guest.mac()
                );
                io::stdout().flush().unwrap();
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;

                if confirm.trim().to_lowercase() == "y" {
                    println!("Unauthorizing guest...");
                    // Use the UnauthorizeGuestBuilder
                    guest_handler
                        .unauthorize(selected_guest.mac())
                        .send()
                        .await?;
                    println!("✅ Guest unauthorized successfully.");
                } else {
                    println!("Operation cancelled.");
                }
            }
            "5" => {
                println!("\nUnauthorize All Guests");
                println!("⚠️☠️🚨  WARNING: This will unauthorize all guests in the system!");
                println!("To confirm, please type UNAUTHORIZE in all caps: ");
                io::stdout().flush().unwrap();

                let mut confirmation = String::new();
                io::stdin().read_line(&mut confirmation)?;

                if confirmation.trim() == "UNAUTHORIZE" {
                    // Use the UnauthorizeAllGuestsBuilder
                    let result = guest_handler.unauthorize_all().send().await;

                    match result {
                        Ok(_) => println!("✅ Successfully unauthorized all guests."),
                        Err(e) => println!("❌ Failed to unauthorize all guests: {}", e),
                    }
                } else {
                    println!("Operation cancelled - confirmation did not match.");
                }
            }
            "6" => {
                println!("\nExiting...");
                break;
            }
            _ => println!("Invalid option. Please try again."),
        }
    }

    Ok(())
}
