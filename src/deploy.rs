use crate::types::*;
use crate::validation::*;
use crate::S3VectorsClient;
use crate::HTTP_CLIENT;
use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;
const MAX_BACKOFF_MS: u64 = 5000;
const MAX_BATCH_SIZE: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum S3VectorsError {
    #[error("Authentication required: {0}")]
    AuthRequired(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Rate limit exceeded, retry after {0}ms")]
    RateLimit(u64),
    
    #[error("Service error: {0}")]
    ServiceError(String),
    
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl S3VectorsClient {
    async fn execute_request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<impl Serialize>,
    ) -> Result<T, S3VectorsError> {
        let url = format!("{}{}", self.endpoint, path);
        
        let signer = self.signer.as_ref()
            .ok_or_else(|| S3VectorsError::AuthRequired("No credentials configured".to_string()))?;
        
        let mut retries = 0;
        let mut backoff = INITIAL_BACKOFF_MS;
        
        loop {
            let mut request = HTTP_CLIENT.request(method.clone(), &url);
            
            // Add body if present
            let body_bytes = if let Some(ref body) = body {
                let bytes = serde_json::to_vec(body)?;
                request = request.body(bytes.clone());
                bytes
            } else {
                vec![]
            };
            
            // Sign the request
            let headers = signer.sign_request(
                &method.as_str(),
                &url,
                HashMap::new(),
                &body_bytes,
            ).await?;
            
            for (key, value) in headers {
                request = request.header(key, value);
            }
            
            request = request.header("Content-Type", "application/x-amz-json-1.0");
            
            debug!("Executing {} request to {}", method, path);
            let response = request.send().await?;
            let status = response.status();
            
            if status.is_success() {
                let result = response.json::<T>().await
                    .context("Failed to parse response")?;
                return Ok(result);
            }
            
            // Handle errors
            let error_text = response.text().await.unwrap_or_default();
            
            if let Ok(service_error) = serde_json::from_str::<ServiceError>(&error_text) {
                match status {
                    StatusCode::NOT_FOUND => {
                        return Err(S3VectorsError::NotFound(service_error.message));
                    }
                    StatusCode::CONFLICT => {
                        return Err(S3VectorsError::AlreadyExists(service_error.message));
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
                        if retries < MAX_RETRIES {
                            warn!("Rate limited, retrying after {}ms", backoff);
                            sleep(Duration::from_millis(backoff)).await;
                            backoff = (backoff * 2).min(MAX_BACKOFF_MS);
                            retries += 1;
                            continue;
                        }
                        return Err(S3VectorsError::RateLimit(backoff));
                    }
                    _ if status.is_server_error() && retries < MAX_RETRIES => {
                        warn!("Server error, retrying after {}ms", backoff);
                        sleep(Duration::from_millis(backoff)).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF_MS);
                        retries += 1;
                        continue;
                    }
                    _ => {
                        return Err(S3VectorsError::ServiceError(service_error.message));
                    }
                }
            }
            
            return Err(S3VectorsError::ServiceError(format!(
                "Request failed with status {}: {}",
                status, error_text
            )));
        }
    }
    
    // Bucket operations
    pub async fn create_vector_bucket(&self, bucket_name: &str) -> Result<VectorBucket, S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        let request = CreateVectorBucketRequest {
            bucket_name: bucket_name.to_string(),
        };
        
        info!("Creating vector bucket: {}", bucket_name);
        let response: CreateVectorBucketResponse = self.execute_request(
            reqwest::Method::POST,
            "/v1/vectorbuckets",
            Some(request),
        ).await?;
        
        Ok(response.bucket)
    }
    
    pub async fn delete_vector_bucket(&self, bucket_name: &str) -> Result<(), S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Deleting vector bucket: {}", bucket_name);
        self.execute_request::<serde_json::Value>(
            reqwest::Method::DELETE,
            &format!("/v1/vectorbuckets/{}", bucket_name),
            None::<()>,
        ).await?;
        
        Ok(())
    }
    
    pub async fn list_vector_buckets(
        &self,
        max_results: Option<u32>,
        next_token: Option<String>,
    ) -> Result<ListVectorBucketsResponse, S3VectorsError> {
        let request = ListVectorBucketsRequest {
            max_results,
            next_token,
        };
        
        info!("Listing vector buckets");
        self.execute_request(
            reqwest::Method::POST,
            "/v1/vectorbuckets/list",
            Some(request),
        ).await
    }
    
    pub async fn describe_vector_bucket(&self, bucket_name: &str) -> Result<VectorBucket, S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Describing vector bucket: {}", bucket_name);
        self.execute_request(
            reqwest::Method::GET,
            &format!("/v1/vectorbuckets/{}", bucket_name),
            None::<()>,
        ).await
    }
    
    // Index operations
    pub async fn create_index(&self, request: CreateIndexRequest) -> Result<VectorIndex, S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_dimensions(request.vector_dimensions)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Creating index {} in bucket {}", request.index_name, request.bucket_name);
        self.execute_request(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes", request.bucket_name),
            Some(request),
        ).await
    }
    
    pub async fn delete_index(&self, bucket_name: &str, index_name: &str) -> Result<(), S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Deleting index {} from bucket {}", index_name, bucket_name);
        self.execute_request::<serde_json::Value>(
            reqwest::Method::DELETE,
            &format!("/v1/vectorbuckets/{}/indexes/{}", bucket_name, index_name),
            None::<()>,
        ).await?;
        
        Ok(())
    }
    
    pub async fn list_indexes(
        &self,
        bucket_name: &str,
        max_results: Option<u32>,
        next_token: Option<String>,
    ) -> Result<ListIndexesResponse, S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        let request = ListIndexesRequest {
            bucket_name: bucket_name.to_string(),
            max_results,
            next_token,
        };
        
        info!("Listing indexes in bucket {}", bucket_name);
        self.execute_request(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/list", bucket_name),
            Some(request),
        ).await
    }
    
    pub async fn describe_index(&self, bucket_name: &str, index_name: &str) -> Result<VectorIndex, S3VectorsError> {
        validate_bucket_name(bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Describing index {} in bucket {}", index_name, bucket_name);
        self.execute_request(
            reqwest::Method::GET,
            &format!("/v1/vectorbuckets/{}/indexes/{}", bucket_name, index_name),
            None::<()>,
        ).await
    }
    
    // Vector operations
    pub async fn put_vectors(&self, request: PutVectorsRequest) -> Result<(), S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        if request.vectors.is_empty() {
            return Err(S3VectorsError::Validation("No vectors provided".to_string()));
        }
        
        if request.vectors.len() > MAX_BATCH_SIZE {
            return Err(S3VectorsError::Validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.vectors.len(),
                MAX_BATCH_SIZE
            )));
        }
        
        info!("Putting {} vectors to index {} in bucket {}", 
            request.vectors.len(), request.index_name, request.bucket_name);
        
        self.execute_request::<serde_json::Value>(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/{}/vectors", 
                request.bucket_name, request.index_name),
            Some(request),
        ).await?;
        
        Ok(())
    }
    
    pub async fn get_vectors(&self, request: GetVectorsRequest) -> Result<GetVectorsResponse, S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        if request.keys.is_empty() {
            return Err(S3VectorsError::Validation("No keys provided".to_string()));
        }
        
        info!("Getting {} vectors from index {} in bucket {}", 
            request.keys.len(), request.index_name, request.bucket_name);
        
        self.execute_request(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/{}/vectors/get", 
                request.bucket_name, request.index_name),
            Some(request),
        ).await
    }
    
    pub async fn delete_vectors(&self, request: DeleteVectorsRequest) -> Result<(), S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        if request.keys.is_empty() {
            return Err(S3VectorsError::Validation("No keys provided".to_string()));
        }
        
        info!("Deleting {} vectors from index {} in bucket {}", 
            request.keys.len(), request.index_name, request.bucket_name);
        
        self.execute_request::<serde_json::Value>(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/{}/vectors/delete", 
                request.bucket_name, request.index_name),
            Some(request),
        ).await?;
        
        Ok(())
    }
    
    pub async fn list_vectors(
        &self,
        request: ListVectorsRequest,
    ) -> Result<ListVectorsResponse, S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Listing vectors in index {} of bucket {}", 
            request.index_name, request.bucket_name);
        
        self.execute_request(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/{}/vectors/list", 
                request.bucket_name, request.index_name),
            Some(request),
        ).await
    }
    
    pub async fn query_vectors(&self, request: QueryVectorsRequest) -> Result<QueryVectorsResponse, S3VectorsError> {
        validate_bucket_name(&request.bucket_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        validate_index_name(&request.index_name)
            .map_err(|e| S3VectorsError::Validation(e.to_string()))?;
        
        info!("Querying vectors in index {} of bucket {}", 
            request.index_name, request.bucket_name);
        
        self.execute_request(
            reqwest::Method::POST,
            &format!("/v1/vectorbuckets/{}/indexes/{}/query", 
                request.bucket_name, request.index_name),
            Some(request),
        ).await
    }
}

// Helper functions
pub async fn create_bucket_and_index(
    client: &S3VectorsClient,
    bucket_name: &str,
    index_name: &str,
    dimensions: u32,
    distance_metric: DistanceMetric,
) -> Result<(VectorBucket, VectorIndex)> {
    info!("Creating bucket {} and index {}", bucket_name, index_name);
    
    // Create bucket
    let _bucket = match client.create_vector_bucket(bucket_name).await {
        Ok(b) => b,
        Err(S3VectorsError::AlreadyExists(_)) => {
            info!("Bucket {} already exists, using existing", bucket_name);
            client.describe_vector_bucket(bucket_name).await?
        }
        Err(e) => return Err(e.into()),
    };
    
    // Wait for bucket to be active
    let bucket = wait_for_bucket_active(client, bucket_name).await?;
    
    // Create index
    let index_request = CreateIndexRequest {
        bucket_name: bucket_name.to_string(),
        index_name: index_name.to_string(),
        vector_dimensions: dimensions,
        distance_metric,
        metadata_fields: None,
    };
    
    let _index = match client.create_index(index_request).await {
        Ok(i) => i,
        Err(S3VectorsError::AlreadyExists(_)) => {
            info!("Index {} already exists, using existing", index_name);
            client.describe_index(bucket_name, index_name).await?
        }
        Err(e) => return Err(e.into()),
    };
    
    // Wait for index to be active
    let index = wait_for_index_active(client, bucket_name, index_name).await?;
    
    Ok((bucket, index))
}

pub async fn batch_put_vectors(
    client: &S3VectorsClient,
    bucket_name: &str,
    index_name: &str,
    vectors: Vec<Vector>,
    expected_dimensions: u32,
) -> Result<()> {
    // Validate all vectors
    for vector in &vectors {
        vector.validate(expected_dimensions)?;
    }
    
    // Process in batches
    for chunk in vectors.chunks(MAX_BATCH_SIZE) {
        let request = PutVectorsRequest {
            bucket_name: bucket_name.to_string(),
            index_name: index_name.to_string(),
            vectors: chunk.to_vec(),
        };
        
        client.put_vectors(request).await?;
        
        // Small delay between batches to avoid rate limiting
        if vectors.len() > MAX_BATCH_SIZE {
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    info!("Successfully put {} vectors", vectors.len());
    Ok(())
}

async fn wait_for_bucket_active(
    client: &S3VectorsClient,
    bucket_name: &str,
) -> Result<VectorBucket> {
    info!("Waiting for bucket {} to become active", bucket_name);
    
    for _ in 0..60 {
        let bucket = client.describe_vector_bucket(bucket_name).await?;
        match bucket.status {
            BucketStatus::Active => {
                info!("Bucket {} is active", bucket_name);
                return Ok(bucket);
            }
            BucketStatus::Failed => {
                return Err(anyhow::anyhow!("Bucket creation failed"));
            }
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    Err(anyhow::anyhow!("Timeout waiting for bucket to become active"))
}

async fn wait_for_index_active(
    client: &S3VectorsClient,
    bucket_name: &str,
    index_name: &str,
) -> Result<VectorIndex> {
    info!("Waiting for index {} to become active", index_name);
    
    for _ in 0..60 {
        let index = client.describe_index(bucket_name, index_name).await?;
        match index.status {
            IndexStatus::Active => {
                info!("Index {} is active", index_name);
                return Ok(index);
            }
            IndexStatus::Failed => {
                return Err(anyhow::anyhow!("Index creation failed"));
            }
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    Err(anyhow::anyhow!("Timeout waiting for index to become active"))
}