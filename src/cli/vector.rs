use crate::cli::output::{print_output, print_table};
use crate::cli::OutputFormat;
use crate::{DeleteVectorsRequest, GetVectorsRequest, ListVectorsRequest, PutVectorsRequest, QueryVector, QueryVectorsRequest, S3VectorsClient, Vector, VectorData};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::fs;
use tabled::Tabled;

#[derive(Args, Debug)]
pub struct VectorCommand {
    #[command(subcommand)]
    pub command: VectorSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum VectorSubcommands {
    #[command(about = "Put vectors into an index")]
    Put {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        index: String,
        
        #[arg(help = "Vector key")]
        key: String,
        
        #[arg(short, long, help = "Vector data as comma-separated floats")]
        data: String,
        
        #[arg(short, long, help = "Metadata as JSON")]
        metadata: Option<String>,
        
        #[arg(short, long, help = "Batch input file (JSON array of vectors)")]
        file: Option<String>,
    },
    
    #[command(about = "Get vectors by keys")]
    Get {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        index: String,
        
        #[arg(help = "Vector keys to retrieve", value_delimiter = ',')]
        keys: Vec<String>,
        
        #[arg(long, help = "Include vector data in response")]
        include_data: bool,
        
        #[arg(long, help = "Include metadata in response")]
        include_metadata: bool,
    },
    
    #[command(about = "List vectors in an index")]
    List {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        index: String,
        
        #[arg(short, long, help = "Maximum number of results", default_value = "100")]
        max_results: u32,
        
        #[arg(long, help = "Include vector data in response")]
        include_data: bool,
        
        #[arg(long, help = "Include metadata in response")]
        include_metadata: bool,
    },
    
    #[command(about = "Delete vectors by keys")]
    Delete {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        index: String,
        
        #[arg(help = "Vector keys to delete", value_delimiter = ',')]
        keys: Vec<String>,
        
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
    
    #[command(about = "Query vectors for similarity search")]
    Query {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
        
        #[arg(help = "Name of the index")]
        index: String,
        
        #[arg(short = 'q', long, help = "Query vector as comma-separated floats")]
        vector: String,
        
        #[arg(short, long, help = "Number of results to return", default_value = "10")]
        top_k: u32,
        
        #[arg(short, long, help = "Filter expression as JSON")]
        filter: Option<String>,
        
        #[arg(long, help = "Include distance scores in response")]
        include_distance: bool,
        
        #[arg(long, help = "Include metadata in response")]
        include_metadata: bool,
    },
}

#[derive(Serialize, Tabled)]
struct VectorInfo {
    key: String,
    has_data: String,
    has_metadata: String,
}

#[derive(Serialize, Tabled)]
struct QueryResult {
    key: String,
    distance: String,
    metadata: String,
}

impl VectorCommand {
    pub async fn execute(&self, client: &S3VectorsClient, output_format: OutputFormat) -> Result<()> {
        match &self.command {
            VectorSubcommands::Put { bucket, index, key, data, metadata, file } => {
                self.put_vectors(client, bucket, index, key, data, metadata.as_deref(), file.as_deref(), output_format).await
            }
            VectorSubcommands::Get { bucket, index, keys, include_data, include_metadata } => {
                self.get_vectors(client, bucket, index, keys, *include_data, *include_metadata, output_format).await
            }
            VectorSubcommands::List { bucket, index, max_results, include_data, include_metadata } => {
                self.list_vectors(client, bucket, index, *max_results, *include_data, *include_metadata, output_format).await
            }
            VectorSubcommands::Delete { bucket, index, keys, force } => {
                self.delete_vectors(client, bucket, index, keys, *force, output_format).await
            }
            VectorSubcommands::Query { bucket, index, vector, top_k, filter, include_distance, include_metadata } => {
                self.query_vectors(client, bucket, index, vector, *top_k, filter.as_deref(), *include_distance, *include_metadata, output_format).await
            }
        }
    }
    
    async fn put_vectors(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        index: &str,
        key: &str,
        data: &str,
        metadata: Option<&str>,
        file: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let vectors = if let Some(file_path) = file {
            // Load vectors from file
            let content = fs::read_to_string(file_path)
                .context("Failed to read vector file")?;
            serde_json::from_str::<Vec<Vector>>(&content)
                .context("Failed to parse vector file")?
        } else {
            // Create single vector from command line args
            let float_data: Vec<f32> = data
                .split(',')
                .map(|s| s.trim().parse())
                .collect::<Result<Vec<f32>, _>>()
                .context("Failed to parse vector data")?;
            
            let metadata_value = if let Some(m) = metadata {
                Some(serde_json::from_str(m).context("Failed to parse metadata")?)
            } else {
                None
            };
            
            vec![Vector {
                key: key.to_string(),
                data: VectorData {
                    float32: float_data,
                },
                metadata: metadata_value,
            }]
        };
        
        let request = PutVectorsRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: index.to_string(),
            vectors: vectors.clone(),
        };
        
        if vectors.len() > 10 {
            let pb = ProgressBar::new(vectors.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            
            // Process in batches of 500
            for chunk in vectors.chunks(500) {
                let batch_request = PutVectorsRequest {
                    vector_bucket_name: bucket.to_string(),
                    index_name: index.to_string(),
                    vectors: chunk.to_vec(),
                };
                client.put_vectors(batch_request).await?;
                pb.inc(chunk.len() as u64);
            }
            pb.finish_with_message("Done");
        } else {
            client.put_vectors(request).await?;
        }
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Successfully put {} vector(s)", vectors.len());
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "vectors_added": vectors.len()
                });
                print_output(&result, output_format)?;
            }
        }
        
        Ok(())
    }
    
    async fn get_vectors(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        index: &str,
        keys: &[String],
        include_data: bool,
        include_metadata: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        let request = GetVectorsRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: index.to_string(),
            keys: keys.to_vec(),
            return_vector: include_data,
            return_metadata: include_metadata,
        };
        
        let response = client.get_vectors(request).await?;
        
        match output_format {
            OutputFormat::Table => {
                let vectors: Vec<VectorInfo> = response.vectors
                    .iter()
                    .map(|v| VectorInfo {
                        key: v.key.clone(),
                        has_data: if v.vector.is_some() { "Yes" } else { "No" }.to_string(),
                        has_metadata: if v.metadata.is_some() { "Yes" } else { "No" }.to_string(),
                    })
                    .collect();
                
                print_table(vectors)?;
            }
            _ => print_output(&response, output_format)?,
        }
        
        Ok(())
    }
    
    async fn list_vectors(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        index: &str,
        max_results: u32,
        _include_data: bool,
        _include_metadata: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        let request = ListVectorsRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: index.to_string(),
            max_results: Some(max_results),
            next_token: None,
        };
        
        let response = client.list_vectors(request).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("Found {} vectors", response.keys.len());
                for key in &response.keys {
                    println!("  - {}", key);
                }
                if response.next_token.is_some() {
                    println!("\nMore results available. Use pagination token to continue.");
                }
            }
            _ => print_output(&response, output_format)?,
        }
        
        Ok(())
    }
    
    async fn delete_vectors(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        index: &str,
        keys: &[String],
        force: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !force {
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt(format!("Are you sure you want to delete {} vector(s)?", keys.len()))
                .default(false)
                .interact()?;
            
            if !proceed {
                println!("Operation cancelled");
                return Ok(());
            }
        }
        
        let request = DeleteVectorsRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: index.to_string(),
            keys: keys.to_vec(),
        };
        
        client.delete_vectors(request).await?;
        
        match output_format {
            OutputFormat::Table => {
                println!("✓ Successfully deleted {} vector(s)", keys.len());
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "vectors_deleted": keys.len()
                });
                print_output(&result, output_format)?;
            }
        }
        
        Ok(())
    }
    
    async fn query_vectors(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        index: &str,
        vector: &str,
        top_k: u32,
        filter: Option<&str>,
        include_distance: bool,
        include_metadata: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        let float_data: Vec<f32> = vector
            .split(',')
            .map(|s| s.trim().parse())
            .collect::<Result<Vec<f32>, _>>()
            .context("Failed to parse query vector")?;
        
        let filter_value = if let Some(f) = filter {
            Some(serde_json::from_str(f).context("Failed to parse filter")?)
        } else {
            None
        };
        
        let request = QueryVectorsRequest {
            vector_bucket_name: bucket.to_string(),
            index_name: index.to_string(),
            query_vector: QueryVector {
                float32: float_data,
            },
            top_k,
            filter: filter_value,
            return_metadata: include_metadata,
            return_distance: include_distance,
        };
        
        let response = client.query_vectors(request).await?;
        
        match output_format {
            OutputFormat::Table => {
                let results: Vec<QueryResult> = response.vectors
                    .iter()
                    .map(|v| QueryResult {
                        key: v.key.clone(),
                        distance: v.distance
                            .map(|d| format!("{:.4}", d))
                            .unwrap_or_else(|| "N/A".to_string()),
                        metadata: v.metadata
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| "N/A".to_string()),
                    })
                    .collect();
                
                print_table(results)?;
            }
            _ => print_output(&response, output_format)?,
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
        command: VectorSubcommands,
    }

    #[test]
    fn test_parse_put_vector_command() {
        let args = vec!["test", "put", "my-bucket", "my-index", "key1", "-d", "0.1,0.2,0.3"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            VectorSubcommands::Put { bucket, index, key, data, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(key, "key1");
                assert_eq!(data, "0.1,0.2,0.3");
            }
            _ => panic!("Expected Put command"),
        }
    }

    #[test]
    fn test_parse_get_vectors_command() {
        let args = vec!["test", "get", "my-bucket", "my-index", "key1,key2"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            VectorSubcommands::Get { bucket, index, keys, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(keys, vec!["key1", "key2"]);
            }
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_parse_query_command() {
        let args = vec!["test", "query", "my-bucket", "my-index", "-q", "0.1,0.2,0.3", "--top-k", "10"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            VectorSubcommands::Query { bucket, index, vector, top_k, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(vector, "0.1,0.2,0.3");
                assert_eq!(top_k, 10);
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_parse_delete_vectors_command() {
        let args = vec!["test", "delete", "my-bucket", "my-index", "key1,key2", "--force"];
        let cli = TestCli::parse_from(args);
        
        match cli.command {
            VectorSubcommands::Delete { bucket, index, keys, force } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(keys, vec!["key1", "key2"]);
                assert!(force);
            }
            _ => panic!("Expected Delete command"),
        }
    }
}