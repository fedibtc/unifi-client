use clap::{Parser, Subcommand};
use unifi_client::{UniFiClient, UniFiResult};

mod guests;
mod sites;
mod utils;

use guests::GuestsValidator;
use sites::SitesValidator;

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
    Guests,
    Sites,
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
        Commands::Sites => {
            let validator = SitesValidator::new(unifi_client);
            validator.run_all_validations().await?;
        }
        Commands::Guests => {
            let validator = GuestsValidator::new(unifi_client);
            validator.run_all_validations().await?;
        }
        Commands::All => {
            println!("Running all validators...");
            let site_validator = SitesValidator::new(unifi_client.clone());
            let guest_validator = GuestsValidator::new(unifi_client.clone());

            site_validator.run_all_validations().await?;
            guest_validator.run_all_validations().await?;
        }
    }

    Ok(())
}
