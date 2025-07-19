use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::io::{self, Write};

use crate::{
    S3VectorsClient,
    rag::{RagConfig, RagPipeline, rag_query},
    cli::OutputFormat,
};

#[derive(Args, Debug)]
pub struct RagCommand {
    #[command(subcommand)]
    pub command: RagSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum RagSubcommands {
    #[command(about = "Initialize RAG pipeline (create bucket and index)")]
    Init {
        #[arg(short, long, help = "S3 Vectors bucket name", default_value = "rag-vectors-default")]
        bucket: String,
        
        #[arg(short, long, help = "S3 Vectors index name", default_value = "documents-default")]
        index: String,
    },
    
    #[command(about = "Ingest documents from a directory")]
    Ingest {
        #[arg(short, long, help = "Directory containing documents to ingest")]
        directory: PathBuf,
        
        #[arg(short, long, help = "S3 Vectors bucket name", default_value = "rag-vectors-default")]
        bucket: String,
        
        #[arg(short, long, help = "S3 Vectors index name", default_value = "documents-default")]
        index: String,
    },
    
    #[command(about = "Query the RAG system")]
    Query {
        #[arg(help = "Query text")]
        query: String,
        
        #[arg(short, long, help = "Number of results to return", default_value = "5")]
        top_k: u32,
        
        #[arg(short, long, help = "S3 Vectors bucket name", default_value = "rag-vectors-default")]
        bucket: String,
        
        #[arg(short, long, help = "S3 Vectors index name", default_value = "documents-default")]
        index: String,
    },
    
    #[command(about = "Interactive RAG query mode")]
    Interactive {
        #[arg(short, long, help = "S3 Vectors bucket name", default_value = "rag-vectors-default")]
        bucket: String,
        
        #[arg(short, long, help = "S3 Vectors index name", default_value = "documents-default")]
        index: String,
    },
}

impl RagCommand {
    pub async fn execute(&self, client: &S3VectorsClient, output_format: OutputFormat) -> Result<()> {
        match &self.command {
            RagSubcommands::Init { bucket, index } => {
                self.init_rag(client, bucket, index, output_format).await
            }
            RagSubcommands::Ingest { directory, bucket, index } => {
                self.ingest_documents(client, directory, bucket, index, output_format).await
            }
            RagSubcommands::Query { query, top_k, bucket, index } => {
                self.query_rag(client, query, *top_k, bucket, index, output_format).await
            }
            RagSubcommands::Interactive { bucket, index } => {
                self.interactive_query(client, bucket, index, output_format).await
            }
        }
    }
    
    async fn init_rag(
        &self,
        client: &S3VectorsClient,
        bucket_name: &str,
        index_name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        println!("🚀 {} RAG pipeline...", "Initializing".cyan());
        
        let config = RagConfig {
            bucket_name: bucket_name.to_string(),
            index_name: index_name.to_string(),
            ..Default::default()
        };
        
        let pipeline = RagPipeline::new(config, client.clone());
        
        match pipeline.initialize().await {
            Ok(_) => {
                match output_format {
                    OutputFormat::Table => {
                        println!("✅ {} initialized successfully!", "RAG pipeline".green());
                        println!("   Bucket: {}", bucket_name.cyan());
                        println!("   Index: {}", index_name.cyan());
                        println!("   Region: {}", client.region().cyan());
                    }
                    _ => {
                        let result = serde_json::json!({
                            "status": "success",
                            "bucket": bucket_name,
                            "index": index_name,
                            "region": client.region(),
                        });
                        crate::cli::output::print_output(&result, output_format)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ {} initializing RAG pipeline: {}", "Failed".red(), e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    async fn ingest_documents(
        &self,
        client: &S3VectorsClient,
        directory: &std::path::Path,
        bucket_name: &str,
        index_name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !directory.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {}", directory.display()));
        }
        
        // Pre-flight check: Verify bucket and index exist
        match self.verify_bucket_and_index(client, bucket_name, index_name).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("\n❌ {}", e);
                eprintln!("\n💡 {} Run: {}", "Tip:".yellow(), format!("rag init --bucket {} --index {}", bucket_name, index_name).cyan());
                return Err(anyhow::anyhow!("Pre-flight check failed"));
            }
        }
        
        println!("📄 {} documents from: {}", "Ingesting".cyan(), directory.display());
        
        let config = RagConfig {
            bucket_name: bucket_name.to_string(),
            index_name: index_name.to_string(),
            ..Default::default()
        };
        
        let pipeline = RagPipeline::new(config, client.clone());
        let start = std::time::Instant::now();
        
        match pipeline.ingest_documents(directory).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                match output_format {
                    OutputFormat::Table => {
                        println!("✅ {} completed in {:?}", "Document ingestion".green(), elapsed);
                    }
                    _ => {
                        let result = serde_json::json!({
                            "status": "success",
                            "directory": directory.display().to_string(),
                            "elapsed_seconds": elapsed.as_secs_f64(),
                        });
                        crate::cli::output::print_output(&result, output_format)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ {} ingesting documents: {}", "Failed".red(), e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    async fn verify_bucket_and_index(
        &self,
        client: &S3VectorsClient,
        bucket_name: &str,
        index_name: &str,
    ) -> Result<()> {
        // Check if bucket exists
        match client.describe_vector_bucket(bucket_name).await {
            Ok(_) => {},
            Err(crate::deploy::S3VectorsError::NotFound(_)) => {
                return Err(anyhow::anyhow!(
                    "Bucket '{}' not found. The RAG pipeline needs to be initialized first.", 
                    bucket_name
                ));
            },
            Err(e) => return Err(anyhow::anyhow!("Failed to check bucket: {}", e)),
        }
        
        // Check if index exists
        match client.describe_index(bucket_name, index_name).await {
            Ok(_) => {},
            Err(crate::deploy::S3VectorsError::NotFound(_)) => {
                return Err(anyhow::anyhow!(
                    "Index '{}' not found in bucket '{}'. The RAG pipeline needs to be initialized first.", 
                    index_name, bucket_name
                ));
            },
            Err(e) => return Err(anyhow::anyhow!("Failed to check index: {}", e)),
        }
        
        Ok(())
    }
    
    async fn query_rag(
        &self,
        client: &S3VectorsClient,
        query: &str,
        top_k: u32,
        bucket_name: &str,
        index_name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        // Pre-flight check: Verify bucket and index exist
        match self.verify_bucket_and_index(client, bucket_name, index_name).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("\n❌ {}", e);
                eprintln!("\n💡 {} Run: {}", "Tip:".yellow(), format!("rag init --bucket {} --index {}", bucket_name, index_name).cyan());
                return Err(anyhow::anyhow!("Pre-flight check failed"));
            }
        }
        
        println!("🔍 {} for: {}", "Searching".cyan(), query);
        println!();
        
        let config = RagConfig {
            bucket_name: bucket_name.to_string(),
            index_name: index_name.to_string(),
            ..Default::default()
        };
        
        let pipeline = RagPipeline::new(config, client.clone());
        
        match rag_query(&pipeline, query, top_k).await {
            Ok(response) => {
                match output_format {
                    OutputFormat::Table => {
                        println!("{}", response);
                    }
                    _ => {
                        let result = serde_json::json!({
                            "query": query,
                            "response": response,
                            "top_k": top_k,
                        });
                        crate::cli::output::print_output(&result, output_format)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ {} querying RAG system: {}", "Error".red(), e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    async fn interactive_query(
        &self,
        client: &S3VectorsClient,
        bucket_name: &str,
        index_name: &str,
        _output_format: OutputFormat,
    ) -> Result<()> {
        // Pre-flight check: Verify bucket and index exist
        match self.verify_bucket_and_index(client, bucket_name, index_name).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("\n❌ {}", e);
                eprintln!("\n💡 {} Run: {}", "Tip:".yellow(), format!("rag init --bucket {} --index {}", bucket_name, index_name).cyan());
                return Err(anyhow::anyhow!("Pre-flight check failed"));
            }
        }
        
        println!("🤖 {} Mode", "Interactive RAG Query".cyan().bold());
        println!("   Using bucket: {}", bucket_name.yellow());
        println!("   Using index: {}", index_name.yellow());
        println!();
        println!("Type {} or {} to exit", "'exit'".red(), "'quit'".red());
        println!();
        
        let config = RagConfig {
            bucket_name: bucket_name.to_string(),
            index_name: index_name.to_string(),
            ..Default::default()
        };
        
        let pipeline = RagPipeline::new(config, client.clone());
        let stdin = io::stdin();
        let mut input = String::new();
        
        loop {
            print!("{} ", "rag>".green().bold());
            io::stdout().flush()?;
            
            input.clear();
            stdin.read_line(&mut input)?;
            
            let query = input.trim();
            
            if query.is_empty() {
                continue;
            }
            
            if query == "exit" || query == "quit" {
                println!("👋 {}", "Goodbye!".yellow());
                break;
            }
            
            match rag_query(&pipeline, query, 5).await {
                Ok(response) => {
                    println!();
                    println!("{}", response);
                    println!();
                }
                Err(e) => {
                    eprintln!("❌ {}: {}", "Error".red(), e);
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    
    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(subcommand)]
        command: RagSubcommands,
    }
    
    #[test]
    fn test_parse_rag_init() {
        let args = vec!["test", "init"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            RagSubcommands::Init { bucket, index } => {
                assert_eq!(bucket, "rag-vectors-default");
                assert_eq!(index, "documents-default");
            }
            _ => panic!("Expected init command"),
        }
    }
    
    #[test]
    fn test_parse_rag_init_with_options() {
        let args = vec!["test", "init", "--bucket", "my-bucket", "--index", "my-index"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            RagSubcommands::Init { bucket, index } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
            }
            _ => panic!("Expected init command"),
        }
    }
    
    #[test]
    fn test_parse_rag_ingest() {
        let args = vec!["test", "ingest", "--directory", "/tmp/docs"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            RagSubcommands::Ingest { directory, bucket, index } => {
                assert_eq!(directory.to_str().unwrap(), "/tmp/docs");
                assert_eq!(bucket, "rag-vectors-default");
                assert_eq!(index, "documents-default");
            }
            _ => panic!("Expected ingest command"),
        }
    }
    
    #[test]
    fn test_parse_rag_query() {
        let args = vec!["test", "query", "What is S3 Vectors?"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            RagSubcommands::Query { query, top_k, .. } => {
                assert_eq!(query, "What is S3 Vectors?");
                assert_eq!(top_k, 5);
            }
            _ => panic!("Expected query command"),
        }
    }
    
    #[test]
    fn test_parse_rag_query_with_options() {
        let args = vec!["test", "query", "How does it work?", "--top-k", "10", "--bucket", "custom"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            RagSubcommands::Query { query, top_k, bucket, .. } => {
                assert_eq!(query, "How does it work?");
                assert_eq!(top_k, 10);
                assert_eq!(bucket, "custom");
            }
            _ => panic!("Expected query command"),
        }
    }
}