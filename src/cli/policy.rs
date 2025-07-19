use crate::cli::output::print_output;
use crate::cli::OutputFormat;
use crate::S3VectorsClient;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::fs;

#[derive(Args, Debug)]
pub struct PolicyCommand {
    #[command(subcommand)]
    pub command: PolicySubcommands,
}

#[derive(Subcommand, Debug)]
pub enum PolicySubcommands {
    #[command(about = "Put a bucket policy")]
    Put {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,

        #[arg(short, long, help = "Policy JSON document")]
        policy: Option<String>,

        #[arg(short, long, help = "Path to policy JSON file")]
        file: Option<String>,
    },

    #[command(about = "Get bucket policy")]
    Get {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,
    },

    #[command(about = "Delete bucket policy")]
    Delete {
        #[arg(help = "Name of the vector bucket")]
        bucket: String,

        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

impl PolicyCommand {
    pub async fn execute(
        &self,
        client: &S3VectorsClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match &self.command {
            PolicySubcommands::Put {
                bucket,
                policy,
                file,
            } => {
                self.put_policy(
                    client,
                    bucket,
                    policy.as_deref(),
                    file.as_deref(),
                    output_format,
                )
                .await
            }
            PolicySubcommands::Get { bucket } => {
                self.get_policy(client, bucket, output_format).await
            }
            PolicySubcommands::Delete { bucket, force } => {
                self.delete_policy(client, bucket, *force, output_format)
                    .await
            }
        }
    }

    async fn put_policy(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        policy: Option<&str>,
        file: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let policy_json = match (policy, file) {
            (Some(p), None) => p.to_string(),
            (None, Some(f)) => fs::read_to_string(f).context("Failed to read policy file")?,
            _ => {
                return Err(anyhow::anyhow!(
                    "Either --policy or --file must be provided"
                ))
            }
        };

        // Validate JSON
        let _: serde_json::Value =
            serde_json::from_str(&policy_json).context("Invalid JSON policy")?;

        client
            .put_vector_bucket_policy(bucket, &policy_json)
            .await?;

        match output_format {
            OutputFormat::Table => {
                println!("✓ Bucket policy updated successfully for '{bucket}'");
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "bucket": bucket,
                    "message": "Policy updated"
                });
                print_output(&result, output_format)?;
            }
        }

        Ok(())
    }

    async fn get_policy(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        let policy = client.get_vector_bucket_policy(bucket).await?;

        match output_format {
            OutputFormat::Table => {
                println!("Bucket Policy for '{bucket}':");
                // Pretty print the JSON policy
                let parsed: serde_json::Value = serde_json::from_str(&policy)?;
                println!("{}", serde_json::to_string_pretty(&parsed)?);
            }
            _ => {
                let result = serde_json::json!({
                    "bucket": bucket,
                    "policy": serde_json::from_str::<serde_json::Value>(&policy)?
                });
                print_output(&result, output_format)?;
            }
        }

        Ok(())
    }

    async fn delete_policy(
        &self,
        client: &S3VectorsClient,
        bucket: &str,
        force: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !force {
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt(format!(
                    "Are you sure you want to delete the policy for bucket '{bucket}'?"
                ))
                .default(false)
                .interact()?;

            if !proceed {
                println!("Operation cancelled");
                return Ok(());
            }
        }

        client.delete_vector_bucket_policy(bucket).await?;

        match output_format {
            OutputFormat::Table => {
                println!("✓ Bucket policy deleted successfully for '{bucket}'");
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "bucket": bucket,
                    "message": "Policy deleted"
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
        command: PolicySubcommands,
    }

    #[test]
    fn test_parse_put_policy_command() {
        let args = vec!["test", "put", "my-bucket"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            PolicySubcommands::Put {
                bucket,
                policy,
                file,
            } => {
                assert_eq!(bucket, "my-bucket");
                assert!(policy.is_none());
                assert!(file.is_none());
            }
            _ => panic!("Expected Put command"),
        }
    }

    #[test]
    fn test_parse_put_policy_with_file() {
        let args = vec!["test", "put", "my-bucket", "--file", "policy.json"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            PolicySubcommands::Put {
                bucket,
                policy,
                file,
            } => {
                assert_eq!(bucket, "my-bucket");
                assert!(policy.is_none());
                assert_eq!(file, Some("policy.json".to_string()));
            }
            _ => panic!("Expected Put command"),
        }
    }

    #[test]
    fn test_parse_get_policy_command() {
        let args = vec!["test", "get", "my-bucket"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            PolicySubcommands::Get { bucket } => {
                assert_eq!(bucket, "my-bucket");
            }
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_parse_delete_policy_command() {
        let args = vec!["test", "delete", "my-bucket", "--force"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            PolicySubcommands::Delete { bucket, force } => {
                assert_eq!(bucket, "my-bucket");
                assert!(force);
            }
            _ => panic!("Expected Delete command"),
        }
    }
}
