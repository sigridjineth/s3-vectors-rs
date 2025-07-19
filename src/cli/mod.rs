pub mod bucket;
pub mod index;
pub mod output;
pub mod policy;
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
    pub command: Commands,
    
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
    
    #[arg(
        long,
        global = true,
        help = "Disable SSL certificate verification"
    )]
    pub no_verify_ssl: bool,
    
    #[arg(
        short,
        long,
        global = true,
        help = "Enable verbose output"
    )]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Manage vector buckets")]
    Bucket(bucket::BucketCommand),
    
    #[command(about = "Manage vector indexes")]
    Index(index::IndexCommand),
    
    #[command(about = "Manage vectors")]
    Vector(vector::VectorCommand),
    
    #[command(about = "Manage bucket policies")]
    Policy(policy::PolicyCommand),
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
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