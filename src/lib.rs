mod auth;
mod config;
mod deploy;
mod types;
mod validation;

// RAG modules
pub mod document;
pub mod embeddings;
pub mod rag;

use std::sync::LazyLock;

use anyhow::Result;

pub use crate::config::CONFIG;
pub use crate::types::*;
pub use crate::validation::*;

// Re-export commonly used functions
pub use crate::deploy::{batch_put_vectors, create_bucket_and_index, S3VectorsError};

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client")
});

#[derive(Clone, Debug)]
pub struct S3VectorsClient {
    endpoint: String,
    region: String,
    signer: Option<auth::AwsV4Signer>,
}

impl S3VectorsClient {
    /// Create a new S3 Vectors client for the specified region
    pub fn new(region: &str) -> Self {
        Self {
            endpoint: format!("https://s3vectors.{}.api.aws", region),
            region: region.to_string(),
            signer: None,
        }
    }
    
    /// Get the region this client is configured for
    pub fn region(&self) -> &str {
        &self.region
    }
    
    /// Create a new client with explicit credentials
    pub fn with_credentials(
        region: &str,
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    ) -> Self {
        Self {
            endpoint: format!("https://s3vectors.{}.api.aws", region),
            region: region.to_string(),
            signer: Some(auth::AwsV4Signer::new(
                access_key_id,
                secret_access_key,
                session_token,
                region.to_string(),
            )),
        }
    }
    
    /// Create a client from environment variables
    pub fn from_env() -> Result<Self> {
        let region = CONFIG.aws_region.clone();
        
        let signer = if CONFIG.has_credentials() {
            let access_key = CONFIG.aws_access_key_id.clone()
                .ok_or_else(|| anyhow::anyhow!("AWS_ACCESS_KEY_ID not set"))?;
            let secret_key = CONFIG.aws_secret_access_key.clone()
                .ok_or_else(|| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY not set"))?;
            
            Some(auth::AwsV4Signer::new(
                access_key,
                secret_key,
                CONFIG.aws_session_token.clone(),
                region.clone(),
            ))
        } else {
            None
        };
        
        Ok(Self {
            endpoint: format!("https://s3vectors.{}.api.aws", region),
            region,
            signer,
        })
    }
}