mod auth;
mod config;
mod deploy;
mod types;
mod validation;

// RAG modules
pub mod document;
pub mod embeddings;
pub mod rag;

// CLI module
pub mod cli;

use std::sync::LazyLock;

use anyhow::Result;

pub use crate::config::{CONFIG, get_config};
pub use crate::types::*;
pub use crate::validation::*;

// Re-export commonly used functions
pub use crate::deploy::{batch_put_vectors, create_bucket_and_index, S3VectorsError};

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    match reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            // Log the error but provide a working default client
            eprintln!("Warning: Failed to build custom HTTP client: {}. Using default client.", e);
            reqwest::Client::new()
        }
    }
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
        let config = get_config();
        let region = config.aws_region.clone();
        
        let signer = if config.has_credentials() {
            let access_key = config.aws_access_key_id.clone()
                .ok_or_else(|| anyhow::anyhow!("AWS_ACCESS_KEY_ID not set"))?;
            let secret_key = config.aws_secret_access_key.clone()
                .ok_or_else(|| anyhow::anyhow!("AWS_SECRET_ACCESS_KEY not set"))?;
            
            Some(auth::AwsV4Signer::new(
                access_key,
                secret_key,
                config.aws_session_token.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_vectors_client_creation() {
        // Test that client can be created with region
        let client = S3VectorsClient::new("us-east-1");
        assert_eq!(client.region(), "us-east-1");
        assert_eq!(client.endpoint, "https://s3vectors.us-east-1.api.aws");
        assert!(client.signer.is_none());
    }

    #[test]
    fn test_from_env_without_credentials() {
        // Should not panic even without credentials
        let result = S3VectorsClient::from_env();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert!(client.signer.is_none());
    }
}