use anyhow::Result;
use clap::{Parser, Subcommand};
use s3_vectors::{
    rag::{RagConfig, RagPipeline, rag_query},
    S3VectorsClient,
};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(author, version, about = "S3 Vectors RAG Demo", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// AWS region for S3 Vectors
    #[arg(short, long, default_value = "us-east-1")]
    region: String,
    
    /// S3 Vectors bucket name
    #[arg(short, long, default_value = "rag-demo-sigrid")]
    bucket: String,
    
    /// S3 Vectors index name
    #[arg(short, long, default_value = "documents-sigrid")]
    index: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the RAG pipeline (create bucket and index)
    Init,
    
    /// Ingest documents from a directory
    Ingest {
        /// Directory containing documents to ingest
        #[arg(short, long)]
        directory: PathBuf,
    },
    
    /// Query the RAG system
    Query {
        /// Query text
        #[arg(short, long)]
        query: String,
        
        /// Number of results to return
        #[arg(short, long, default_value = "5")]
        top_k: u32,
    },
    
    /// Interactive query mode
    Interactive,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    let cli = Cli::parse();
    
    // Create S3 Vectors client
    let client = S3VectorsClient::from_env()?;
    
    // Create RAG config
    let config = RagConfig {
        bucket_name: cli.bucket.clone(),
        index_name: cli.index.clone(),
        ..Default::default()
    };
    
    // Create RAG pipeline
    let pipeline = RagPipeline::new(config, client);
    
    match cli.command {
        Commands::Init => {
            println!("🚀 Initializing RAG pipeline...");
            pipeline.initialize().await?;
            println!("✅ RAG pipeline initialized successfully!");
            println!("   Bucket: {}", cli.bucket);
            println!("   Index: {}", cli.index);
            println!("   Region: {}", cli.region);
        }
        
        Commands::Ingest { directory } => {
            println!("📄 Ingesting documents from: {}", directory.display());
            
            if !directory.exists() {
                eprintln!("❌ Directory does not exist: {}", directory.display());
                std::process::exit(1);
            }
            
            let start = std::time::Instant::now();
            pipeline.ingest_documents(&directory).await?;
            let elapsed = start.elapsed();
            
            println!("✅ Document ingestion completed in {:?}", elapsed);
        }
        
        Commands::Query { query, top_k } => {
            println!("🔍 Searching for: {}", query);
            println!();
            
            let response = rag_query(&pipeline, &query, top_k).await?;
            println!("{}", response);
        }
        
        Commands::Interactive => {
            println!("🤖 Interactive RAG Query Mode");
            println!("Type 'exit' or 'quit' to stop");
            println!();
            
            let stdin = std::io::stdin();
            let mut input = String::new();
            
            loop {
                print!("> ");
                std::io::Write::flush(&mut std::io::stdout())?;
                
                input.clear();
                stdin.read_line(&mut input)?;
                
                let query = input.trim();
                
                if query.is_empty() {
                    continue;
                }
                
                if query == "exit" || query == "quit" {
                    println!("👋 Goodbye!");
                    break;
                }
                
                match rag_query(&pipeline, query, 5).await {
                    Ok(response) => {
                        println!();
                        println!("{}", response);
                        println!();
                    }
                    Err(e) => {
                        eprintln!("❌ Error: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}