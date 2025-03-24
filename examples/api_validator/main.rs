use clap::{Parser, Subcommand};

use unifi_client::{UniFiClient, UniFiResult};

mod guest;
mod site;
// mod voucher;
mod utils;

use guest::GuestValidator;
use site::SiteValidator;
// use voucher::VoucherValidator;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short = 'c', long)]
    controller_url: String,

    #[arg(short = 'u', long)]
    username: String,

    #[arg(short = 'p', long)]
    password: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    All,
    Guest,
    Site,
    // Voucher,
}

#[tokio::main]
async fn main() -> UniFiResult<()> {
    let cli = Cli::parse();

    let unifi_client = UniFiClient::builder()
        .controller_url(&cli.controller_url)
        .username(&cli.username)
        .password(&cli.password)
        .site("default")
        .verify_ssl(false)
        .build()
        .await
        .expect("Failed to build UniFiClient");
    
    match cli.command.unwrap_or(Commands::All) {
        // Commands::Voucher => {
        //     let mut validator = VoucherValidator::new(unifi_client);
        //     validator.run_all_validations().await?;
        // }
        Commands::Site => {
            let validator = SiteValidator::new(unifi_client);
            validator.run_all_validations().await?;
        }
        Commands::Guest => {
            let validator = GuestValidator::new(unifi_client);
            validator.run_all_validations().await?;
        }
        Commands::All => {
            println!("Running all validators...");
            // let mut voucher_validator = VoucherValidator::new(unifi_client.clone());
            let site_validator = SiteValidator::new(unifi_client.clone());
            let guest_validator = GuestValidator::new(unifi_client.clone());
            
            // voucher_validator.run_all_validations().await?;
            site_validator.run_all_validations().await?;
            guest_validator.run_all_validations().await?;
        }
    }

    Ok(())
}