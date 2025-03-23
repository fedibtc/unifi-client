use env_logger;
use std::env;
use std::error::Error;
use std::io::{self, Write};

use unifi_client::{ClientConfig, UniFiClient, VoucherConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enable logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Get configuration from environment variables
    let controller = env::var("UNIFI_CONTROLLER")
        .unwrap_or_else(|_| "https://unifi.example.com:8443".to_string());

    let username = env::var("UNIFI_USERNAME").unwrap_or_else(|_| "admin".to_string());

    let password = env::var("UNIFI_PASSWORD").ok();

    let site = env::var("UNIFI_SITE").unwrap_or_else(|_| "default".to_string());

    let verify_ssl = env::var("UNIFI_VERIFY_SSL")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    println!("UniFi Voucher Management Example");
    println!("================================");
    println!("Controller: {}", controller);
    println!("Site: {}", site);

    // Create client configuration
    let config = ClientConfig::builder()
        .controller_url(&controller)
        .username(&username)
        .site(&site)
        .verify_ssl(verify_ssl)
        .build()?;

    // Create the UniFi client and authenticate
    let mut client = UniFiClient::new(config);
    client.login(password).await?;
    println!("✅ Authentication successful!");

    // Get a reference to the voucher API
    let voucher_api = client.vouchers();

    // Display menu
    loop {
        println!("\nVoucher Management Options:");
        println!("1. List Vouchers");
        println!("2. Create Vouchers");
        println!("3. Delete a Voucher");
        println!("4. Delete All Vouchers");
        println!("5. Exit");
        print!("\nSelect an option (1-5): ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        match choice.trim() {
            "1" => {
                println!("\nFetching vouchers...");
                let vouchers = voucher_api.list().await?;

                if vouchers.is_empty() {
                    println!("No vouchers found.");
                } else {
                    println!("\nFound {} vouchers:", vouchers.len());
                    println!(
                        "{:<10} {:<15} {:<10} {:<12} {}",
                        "ID", "Code", "Duration", "Status", "Note"
                    );
                    println!("{}", "-".repeat(80));

                    for voucher in vouchers {
                        println!(
                            "{:<10} {:<15} {:<10} {:<12} {}",
                            &voucher.id[..8],
                            voucher.code,
                            format!("{}m", voucher.duration),
                            format!("{:?}", voucher.status),
                            voucher.note.unwrap_or_default()
                        );
                    }
                }
            }
            "2" => {
                println!("\nCreating New Vouchers");

                // Get voucher parameters
                print!("Number of vouchers to create: ");
                io::stdout().flush().unwrap();
                let mut count_str = String::new();
                io::stdin().read_line(&mut count_str)?;
                let count: u32 = count_str.trim().parse()?;

                print!("Duration in minutes (e.g., 1440 for 1 day): ");
                io::stdout().flush().unwrap();
                let mut duration_str = String::new();
                io::stdin().read_line(&mut duration_str)?;
                let duration: u32 = duration_str.trim().parse()?;

                print!("Note (optional): ");
                io::stdout().flush().unwrap();
                let mut note = String::new();
                io::stdin().read_line(&mut note)?;
                let note = if note.trim().is_empty() {
                    None
                } else {
                    Some(note.trim().to_string())
                };

                // Create vouchers
                println!(
                    "\nCreating {} vouchers with {} minute duration...",
                    count, duration
                );
                let voucher_config = VoucherConfig::builder()
                    .count(count)
                    .duration(duration)
                    .note(note.unwrap_or_default())
                    .build()?;
                let voucher_create_response = voucher_api.create(voucher_config).await?;
                println!(
                    "\n✅ Successfully created {} voucher{}\n",
                    count,
                    if count > 1 { "s" } else { "" }
                );

                // Print the created vouchers
                let vouchers = voucher_api
                    .get_by_create_time(voucher_create_response.create_time)
                    .await?;
                if vouchers.is_empty() {
                    println!("No vouchers found.");
                } else {
                    println!(
                        "{:<10} {:<15} {:<10} {:<12} {}",
                        "ID", "Code", "Duration", "Status", "Note"
                    );
                    println!("{}", "-".repeat(80));

                    for voucher in vouchers {
                        println!(
                            "{:<10} {:<15} {:<10} {:<12} {}",
                            &voucher.id[..8],
                            voucher.code,
                            format!("{}m", voucher.duration),
                            format!("{:?}", voucher.status),
                            voucher.note.unwrap_or_default()
                        );
                    }
                }
            }
            "3" => {
                println!("\nDelete a Voucher");

                // First list vouchers for selection
                let vouchers = voucher_api.list().await?;

                if vouchers.is_empty() {
                    println!("No vouchers available to delete.");
                    continue;
                }

                println!("\nAvailable vouchers:");
                println!("{:<5} {:<15} {:<12}", "Num", "Code", "Status");
                println!("{}", "-".repeat(40));

                for (i, voucher) in vouchers.iter().enumerate() {
                    println!(
                        "{:<5} {:<15} {:<12}",
                        i + 1,
                        voucher.code,
                        format!("{:?}", voucher.status)
                    );
                }

                print!("\nEnter voucher number to delete (or 0 to cancel): ");
                io::stdout().flush().unwrap();
                let mut selection = String::new();
                io::stdin().read_line(&mut selection)?;
                let selection: usize = selection.trim().parse()?;

                if selection == 0 || selection > vouchers.len() {
                    println!("Operation cancelled or invalid selection.");
                    continue;
                }

                let selected_voucher = &vouchers[selection - 1];

                print!(
                    "Are you sure you want to delete voucher {} (y/n)? ",
                    selected_voucher.code
                );
                io::stdout().flush().unwrap();
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;

                if confirm.trim().to_lowercase() == "y" {
                    println!("Deleting voucher...");
                    voucher_api.delete(&selected_voucher.id).await?;
                    println!("✅ Voucher deleted successfully.");
                } else {
                    println!("Deletion cancelled.");
                }
            }
            "4" => {
                println!("\nDelete All Vouchers");
                println!("⚠️☠️🚨  WARNING: This will delete all vouchers in the system!");
                println!("To confirm, please type DELETE in all caps: ");
                io::stdout().flush().unwrap();
                
                let mut confirmation = String::new();
                io::stdin().read_line(&mut confirmation)?;

                if confirmation.trim() == "DELETE" {
                    println!("Fetching vouchers...");
                    let vouchers = voucher_api.list().await?;
                    
                    if vouchers.is_empty() {
                        println!("No vouchers to delete.");
                        continue;
                    }

                    println!("Deleting {} vouchers...", vouchers.len());
                    for voucher in vouchers {
                        voucher_api.delete(&voucher.id).await?;
                    }
                    println!("✅ Successfully deleted all vouchers.");
                } else {
                    println!("Deletion cancelled - confirmation did not match.");
                }
            }
            "5" => {
                println!("\nExiting...");
                break;
            }
            _ => println!("Invalid option. Please try again."),
        }
    }

    Ok(())
}
