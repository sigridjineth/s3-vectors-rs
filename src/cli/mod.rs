pub mod bucket;
pub mod index;
pub mod init;
pub mod install_models;
pub mod interactive;
pub mod output;
pub mod policy;
pub mod rag;
pub mod vector;

use clap::{Parser, Subcommand};
use std::fmt;

#[derive(Parser, Debug)]
#[command(
    name = "s3-vectors",
    version,
    about = "AWS S3 Vectors CLI - Manage vector storage and similarity search",
    long_about = "A comprehensive CLI tool for managing AWS S3 Vectors service.\n\
                  Store, manage, and query high-dimensional vectors for ML applications."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(
        short,
        long,
        global = true,
        help = "AWS region",
        env = "AWS_REGION",
        default_value = "us-east-1"
    )]
    pub region: String,

    #[arg(
        short,
        long,
        global = true,
        help = "AWS profile to use",
        env = "AWS_PROFILE"
    )]
    pub profile: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        global = true,
        help = "Output format",
        default_value = "table"
    )]
    pub output: OutputFormat,

    #[arg(long, global = true, help = "Disable SSL certificate verification")]
    pub no_verify_ssl: bool,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Initialize AWS credentials")]
    Init(init::InitCommand),

    #[command(about = "Download ML models for RAG functionality")]
    InstallModels(install_models::InstallModelsCommand),

    #[command(about = "Manage vector buckets")]
    Bucket(bucket::BucketCommand),

    #[command(about = "Manage vector indexes")]
    Index(index::IndexCommand),

    #[command(about = "Manage vectors")]
    Vector(vector::VectorCommand),

    #[command(about = "Manage bucket policies")]
    Policy(policy::PolicyCommand),

    #[command(about = "RAG (Retrieval-Augmented Generation) operations")]
    Rag(rag::RagCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Yaml,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Yaml => write!(f, "yaml"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_cli_without_command() {
        // Test that CLI can be parsed without a command (for interactive mode)
        let args = vec!["s3-vectors"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_parse_cli_with_bucket_command() {
        let args = vec!["s3-vectors", "bucket", "list"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(matches!(cli.command, Some(Commands::Bucket(_))));
    }

    #[test]
    fn test_parse_cli_with_global_options() {
        let args = vec![
            "s3-vectors",
            "--region",
            "us-west-2",
            "--verbose",
            "bucket",
            "list",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.region, "us-west-2");
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Bucket(_))));
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Yaml.to_string(), "yaml");
    }
}
