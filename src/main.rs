use anyhow::Result;
use clap::Parser;
use s3_vectors::cli::{Cli, Commands, interactive::InteractiveMode};
use s3_vectors::S3VectorsClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Initialize logging based on verbosity
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Create S3 Vectors client with proper precedence: profile > env > default
    let client = match (&cli.profile, S3VectorsClient::from_env_with_region(Some(&cli.region))) {
        (Some(profile), _) => {
            tracing::info!("Using AWS profile: {}", profile);
            S3VectorsClient::from_profile(profile, &cli.region)
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load profile '{}': {}. Using default client.", profile, e);
                    S3VectorsClient::new(&cli.region)
                })
        },
        (None, Ok(client)) => client,
        (None, Err(_)) => {
            tracing::debug!("No credentials found in environment, using anonymous client");
            S3VectorsClient::new(&cli.region)
        },
    };
    
    
    // Execute the appropriate command or enter interactive mode
    match &cli.command {
        Some(Commands::Init(cmd)) => cmd.execute().await?,
        Some(Commands::InstallModels(cmd)) => cmd.execute().await?,
        Some(Commands::Bucket(cmd)) => cmd.execute(&client, cli.output).await?,
        Some(Commands::Index(cmd)) => cmd.execute(&client, cli.output).await?,
        Some(Commands::Vector(cmd)) => cmd.execute(&client, cli.output).await?,
        Some(Commands::Policy(cmd)) => cmd.execute(&client, cli.output).await?,
        None => {
            // Enter interactive mode
            let interactive = InteractiveMode::new(client, cli.output, cli.verbose);
            interactive.run().await?;
        }
    }
    
    Ok(())
}