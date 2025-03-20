use clap::{Parser, Subcommand};

use unifi_client::{ClientConfig, UnifiClient, UnifiResult};

mod guest;
mod site;
mod voucher;
mod utils;

use guest::GuestValidator;
use site::SiteValidator;
use voucher::VoucherValidator;

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
    Voucher,
}

#[tokio::main]
async fn main() -> UnifiResult<()> {
    let cli = Cli::parse();

    let config = ClientConfig::builder()
        .controller_url(&cli.controller_url)
        .username(&cli.username)
        .password(&cli.password)
        .site("default")
        .verify_ssl(false)
        .build()?;

    let mut client = UnifiClient::new(config);
    
    // Login first
    client.login(None).await?;

    match cli.command.unwrap_or(Commands::All) {
        Commands::Voucher => {
            let mut validator = VoucherValidator::new(client);
            validator.run_all_validations().await?;
        }
        Commands::Site => {
            let validator = SiteValidator::new(client);
            validator.run_all_validations().await?;
        }
        Commands::Guest => {
            let validator = GuestValidator::new(client);
            validator.run_all_validations().await?;
        }
        Commands::All => {
            println!("Running all validators...");
            let mut voucher_validator = VoucherValidator::new(client.clone());
            let site_validator = SiteValidator::new(client.clone());
            let guest_validator = GuestValidator::new(client.clone());
            
            voucher_validator.run_all_validations().await?;
            site_validator.run_all_validations().await?;
            guest_validator.run_all_validations().await?;
        }
    }

    Ok(())
}