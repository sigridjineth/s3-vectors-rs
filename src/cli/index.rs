use crate::cli::output::{print_output, print_table};
use crate::cli::OutputFormat;
use crate::{CreateIndexRequest, DistanceMetric, S3VectorsClient};
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

#[derive(Args, Debug)]
pub struct IndexCommand {
    #[command(subcommand)]
    pub command: IndexSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum IndexSubcommands {
    #[command(about = "Create a new vector index")]
    Create {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        name: String,
        
        #[arg(short, long, help = "Number of dimensions (1-4096)")]
        dimensions: u32,
        
        #[arg(short, long, value_enum, help = "Distance metric", default_value = "cosine")]
        metric: DistanceMetricArg,
        
        #[arg(long, help = "Metadata fields configuration as JSON")]
        metadata_config: Option<String>,
    },
    
    #[command(about = "List indexes in a bucket")]
    List {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(short, long, help = "Maximum number of results", default_value = "100")]
        max_results: u32,
        
        #[arg(long, help = "Prefix to filter index names")]
        prefix: Option<String>,
    },
    
    #[command(about = "Get index details")]
    Get {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        name: String,
    },
    
    #[command(about = "Delete an index")]
    Delete {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        name: String,
        
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq)]
pub enum DistanceMetricArg {
    Euclidean,
    Cosine,
}

impl From<DistanceMetricArg> for DistanceMetric {
    fn from(arg: DistanceMetricArg) -> Self {
        match arg {
            DistanceMetricArg::Euclidean => DistanceMetric::Euclidean,
            DistanceMetricArg::Cosine => DistanceMetric::Cosine,
        }
    }
}

#[derive(Serialize, Tabled)]
struct IndexInfo {
    name: String,
    dimensions: u32,
    metric: String,
    status: String,
    vectors: String,
}

impl IndexCommand {
    pub async fn execute(&self, client: &S3VectorsClient, output_format: OutputFormat) -> Result<()> {
        match &self.command {
            IndexSubcommands::Create { bucket, name, dimensions, metric, metadata_config } => {
                self.create_index(client, bucket, name, *dimensions, *metric, metadata_config.as_deref(), output_format).await
            }
            IndexSubcommands::List { bucket, max_results, prefix } => {
                self.list_indexes(client, bucket, *max_results, prefix.as_deref(), output_format).await
            }
            IndexSubcommands::Get { bucket, name } => {
                self.get_index(client, bucket, name, output_format).await
            }
            IndexSubcommands::Delete { bucket, name, force } => {
                self.delete_index(client, bucket, name, *force, output_format).await
            }
        }
    }
    
    async fn create_index(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        name: &str,
        dimensions: u32,
        metric: DistanceMetricArg,
        metadata_config: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let mut request = CreateIndexRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: name.to_string(),
            data_type: crate::DataType::Float32,
            dimension: dimensions,
            distance_metric: metric.into(),
            metadata_configuration: None,
        };
        
        if let Some(config) = metadata_config {
            request.metadata_configuration = Some(serde_json::from_str(config)?);
        }
        
        client.create_index(request).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Index created successfully");
                println!("Bucket: {}", bucket);
                println!("Index: {}", name);
                println!("Dimensions: {}", dimensions);
                println!("Metric: {:?}", metric);
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "bucket": bucket,
                    "index": name,
                    "dimensions": dimensions,
                    "metric": format!("{:?}", metric)
                });
                print_output(&result, output_format)?;
            }
        }
        
        Ok(())
    }
    
    async fn list_indexes(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        max_results: u32,
        _prefix: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let response = client.list_indexes(bucket, Some(max_results), None).await?;
        
        match output_format {
            OutputFormat::Table => {
                let indexes: Vec<IndexInfo> = response.indexes
                    .iter()
                    .map(|idx| IndexInfo {
                        name: idx.index_name.clone(),
                        dimensions: idx.dimension,
                        metric: format!("{:?}", idx.distance_metric),
                        status: format!("{:?}", idx.status),
                        vectors: idx.vector_count
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    })
                    .collect();
                
                print_table(indexes)?;
            }
            _ => print_output(&response, output_format)?,
        }
        
        Ok(())
    }
    
    async fn get_index(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        let response = client.get_index(bucket, name).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("Index Details:");
                println!("  Name: {}", response.index.index_name);
                println!("  Bucket: {}", response.index.vector_bucket_name);
                println!("  Status: {:?}", response.index.status);
                println!("  Dimensions: {}", response.index.dimension);
                if let Some(count) = response.index.vector_count {
                    println!("  Vectors: {}", count);
                }
                println!("  Created: {}", response.index.created_at);
            }
            _ => print_output(&response.index, output_format)?,
        }
        
        Ok(())
    }
    
    async fn delete_index(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        name: &str,
        force: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !force {
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt(format!("Are you sure you want to delete index '{}/{}'?", bucket, name))
                .default(false)
                .interact()?;
            
            if !proceed {
                println!("Operation cancelled");
                return Ok(());
            }
        }
        
        client.delete_index(bucket, name).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Index '{}/{}' deleted successfully", bucket, name);
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "message": format!("Index '{}/{}' deleted", bucket, name)
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
        command: IndexSubcommands,
    }

    #[test]
    fn test_parse_create_index_command() {
        let args = vec!["test", "create", "my-bucket", "my-index", "--dimensions", "384"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            IndexSubcommands::Create { bucket, name, dimensions, metric, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(name, "my-index");
                assert_eq!(dimensions, 384);
                assert_eq!(metric, DistanceMetricArg::Cosine);
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_parse_list_indexes_command() {
        let args = vec!["test", "list", "my-bucket"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            IndexSubcommands::List { bucket, max_results, prefix } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(max_results, 100); // default
                assert!(prefix.is_none());
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_parse_get_index_command() {
        let args = vec!["test", "get", "my-bucket", "my-index"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            IndexSubcommands::Get { bucket, name } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(name, "my-index");
            }
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_parse_delete_index_command() {
        let args = vec!["test", "delete", "my-bucket", "my-index", "--force"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            IndexSubcommands::Delete { bucket, name, force } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(name, "my-index");
                assert!(force);
            }
            _ => panic!("Expected Delete command"),
        }
    }
}