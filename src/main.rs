use anyhow::Result;
use clap::Parser;
use s3_vectors::cli::{Cli, Commands};
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
    
    // Create S3 Vectors client
    let client = if let Some(_profile) = &cli.profile {
        // TODO: Implement AWS profile support
        tracing::warn!("AWS profile support not yet implemented, using environment credentials");
        S3VectorsClient::from_env()
            .unwrap_or_else(|_| S3VectorsClient::new(&cli.region))
    } else {
        S3VectorsClient::from_env()
            .unwrap_or_else(|_| S3VectorsClient::new(&cli.region))
    };
    
    // Verify region matches
    if client.region() != cli.region && std::env::var("AWS_REGION").is_err() {
        tracing::warn!(
            "Client region '{}' doesn't match CLI region '{}'. Using client region.",
            client.region(),
            cli.region
        );
    }
    
    // Execute the appropriate command
    match &cli.command {
        Commands::Bucket(cmd) => cmd.execute(&client, cli.output).await?,
        Commands::Index(cmd) => cmd.execute(&client, cli.output).await?,
        Commands::Vector(cmd) => cmd.execute(&client, cli.output).await?,
        Commands::Policy(cmd) => cmd.execute(&client, cli.output).await?,
    }
    
    Ok(())
}