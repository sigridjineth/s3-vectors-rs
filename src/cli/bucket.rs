use crate::cli::output::{print_output, print_table};
use crate::cli::OutputFormat;
use crate::S3VectorsClient;
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

#[derive(Args, Debug)]
pub struct BucketCommand {
    #[command(subcommand)]
    pub command: BucketSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum BucketSubcommands {
    #[command(about = "Create a new vector bucket")]
    Create {
        #[arg(help = "Name of the vector bucket")]
        name: String,
        
        #[arg(long, help = "KMS key ID for encryption")]
        kms_key_id: Option<String>,
        
        #[arg(long, help = "Tags in key=value format", value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
    
    #[command(about = "List vector buckets")]
    List {
        #[arg(short, long, help = "Maximum number of results", default_value = "100")]
        max_results: u32,
        
        #[arg(short, long, help = "Prefix to filter bucket names")]
        prefix: Option<String>,
    },
    
    #[command(about = "Get vector bucket details")]
    Get {
        #[arg(help = "Name of the vector bucket")]
        name: String,
    },
    
    #[command(about = "Delete a vector bucket")]
    Delete {
        #[arg(help = "Name of the vector bucket")]
        name: String,
        
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

#[derive(Serialize, Tabled)]
struct BucketInfo {
    name: String,
    status: String,
    created_at: String,
    region: String,
}

impl BucketCommand {
    pub async fn execute(&self, client: &S3VectorsClient, output_format: OutputFormat) -> Result<()> {
        match &self.command {
            BucketSubcommands::Create { name, kms_key_id, tags } => {
                self.create_bucket(client, name, kms_key_id.as_deref(), tags.as_ref(), output_format).await
            }
            BucketSubcommands::List { max_results, prefix } => {
                self.list_buckets(client, *max_results, prefix.as_deref(), output_format).await
            }
            BucketSubcommands::Get { name } => {
                self.get_bucket(client, name, output_format).await
            }
            BucketSubcommands::Delete { name, force } => {
                self.delete_bucket(client, name, *force, output_format).await
            }
        }
    }
    
    async fn create_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        _kms_key_id: Option<&str>,
        _tags: Option<&Vec<String>>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let bucket = client.create_vector_bucket(name).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Vector bucket created successfully");
                println!("Name: {}", bucket.vector_bucket_name);
                println!("Status: {:?}", bucket.status);
                println!("ARN: {}", bucket.vector_bucket_arn);
            }
            _ => print_output(&bucket, output_format)?,
        }
        
        Ok(())
    }
    
    async fn list_buckets(
        &self,
        client: &S3VectorsClient,
        max_results: u32,
        _prefix: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let response = client.list_vector_buckets(Some(max_results), None).await?;
        
        match output_format {
            OutputFormat::Table => {
                let buckets: Vec<BucketInfo> = response.buckets
                    .iter()
                    .map(|b| BucketInfo {
                        name: b.vector_bucket_name.clone(),
                        status: format!("{:?}", b.status),
                        created_at: chrono::DateTime::from_timestamp(b.creation_time as i64, 0)
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default(),
                        region: client.region().to_string(),
                    })
                    .collect();
                
                print_table(buckets)?;
            }
            _ => print_output(&response, output_format)?,
        }
        
        Ok(())
    }
    
    async fn get_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        let bucket = client.describe_vector_bucket(name).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("Vector Bucket Details:");
                println!("  Name: {}", bucket.vector_bucket_name);
                println!("  ARN: {}", bucket.vector_bucket_arn);
                println!("  Status: {:?}", bucket.status);
                if let Some(created) = chrono::DateTime::from_timestamp(bucket.creation_time as i64, 0) {
                    println!("  Created: {}", created.format("%Y-%m-%d %H:%M:%S"));
                }
                if let Some(encryption) = &bucket.encryption_configuration {
                    if let Some(sse_type) = &encryption.sse_type {
                        println!("  Encryption: {}", sse_type);
                    }
                }
            }
            _ => print_output(&bucket, output_format)?,
        }
        
        Ok(())
    }
    
    async fn delete_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        force: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !force {
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt(format!("Are you sure you want to delete bucket '{}'?", name))
                .default(false)
                .interact()?;
            
            if !proceed {
                println!("Operation cancelled");
                return Ok(());
            }
        }
        
        client.delete_vector_bucket(name).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Vector bucket '{}' deleted successfully", name);
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "message": format!("Vector bucket '{}' deleted", name)
                });
                print_output(&result, output_format)?;
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
        command: BucketSubcommands,
    }

    #[test]
    fn test_parse_create_command() {
        let args = vec!["test", "create", "my-bucket"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            BucketSubcommands::Create { name, kms_key_id, tags } => {
                assert_eq!(name, "my-bucket");
                assert!(kms_key_id.is_none());
                assert!(tags.is_none());
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_parse_create_with_options() {
        let args = vec!["test", "create", "my-bucket", "--kms-key-id", "key123", "--tags", "env=prod,team=data"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            BucketSubcommands::Create { name, kms_key_id, tags } => {
                assert_eq!(name, "my-bucket");
                assert_eq!(kms_key_id, Some("key123".to_string()));
                assert_eq!(tags, Some(vec!["env=prod".to_string(), "team=data".to_string()]));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_parse_list_command() {
        let args = vec!["test", "list"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            BucketSubcommands::List { max_results, prefix } => {
                assert_eq!(max_results, 100); // default value
                assert!(prefix.is_none());
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_parse_get_command() {
        let args = vec!["test", "get", "my-bucket"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            BucketSubcommands::Get { name } => {
                assert_eq!(name, "my-bucket");
            }
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_parse_delete_command() {
        let args = vec!["test", "delete", "my-bucket", "--force"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            BucketSubcommands::Delete { name, force } => {
                assert_eq!(name, "my-bucket");
                assert!(force);
            }
            _ => panic!("Expected Delete command"),
        }
    }
}