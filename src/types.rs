use serde::{Deserialize, Serialize};
use serde_json::Value;

// Enums
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BucketStatus {
    Creating,
    Active,
    Deleting,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IndexStatus {
    Creating,
    Active,
    Deleting,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
}

// Vector bucket types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorBucket {
    pub bucket_name: String,
    pub bucket_arn: String,
    pub region: String,
    pub created_at: String,
    pub status: BucketStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVectorBucketRequest {
    pub bucket_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVectorBucketResponse {
    pub bucket: VectorBucket,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteVectorBucketRequest {
    pub bucket_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorBucketsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorBucketsResponse {
    pub buckets: Vec<VectorBucket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

// Vector index types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorIndex {
    pub index_name: String,
    pub index_arn: String,
    pub vector_dimensions: u32,
    pub distance_metric: DistanceMetric,
    pub created_at: String,
    pub status: IndexStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIndexRequest {
    pub bucket_name: String,
    pub index_name: String,
    pub vector_dimensions: u32,
    pub distance_metric: DistanceMetric,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_fields: Option<Vec<MetadataField>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: MetadataFieldType,
    #[serde(default = "default_true")]
    pub filterable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataFieldType {
    String,
    Number,
    Boolean,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteIndexRequest {
    pub bucket_name: String,
    pub index_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndexesRequest {
    pub bucket_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndexesResponse {
    pub indexes: Vec<VectorIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

// Vector types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vector {
    pub key: String,
    pub vector: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl Vector {
    pub fn validate(&self, expected_dimensions: u32) -> anyhow::Result<()> {
        if self.vector.len() != expected_dimensions as usize {
            anyhow::bail!(
                "Vector dimension mismatch: expected {}, got {}",
                expected_dimensions,
                self.vector.len()
            );
        }
        
        if let Some(ref metadata) = self.metadata {
            let size = serde_json::to_vec(metadata)?.len();
            if size > 40960 {
                anyhow::bail!("Metadata size exceeds 40KB limit: {} bytes", size);
            }
        }
        
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PutVectorsRequest {
    pub bucket_name: String,
    pub index_name: String,
    pub vectors: Vec<Vector>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVectorsRequest {
    pub bucket_name: String,
    pub index_name: String,
    pub keys: Vec<String>,
    #[serde(default = "default_true")]
    pub return_vector: bool,
    #[serde(default = "default_true")]
    pub return_metadata: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVectorsResponse {
    pub vectors: Vec<RetrievedVector>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RetrievedVector {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteVectorsRequest {
    pub bucket_name: String,
    pub index_name: String,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorsRequest {
    pub bucket_name: String,
    pub index_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorsResponse {
    pub keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryVectorsRequest {
    pub bucket_name: String,
    pub index_name: String,
    pub query_vector: QueryVector,
    pub top_k: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
    #[serde(default = "default_true")]
    pub return_metadata: bool,
    #[serde(default = "default_true")]
    pub return_distance: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryVector {
    pub float32: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueryVectorsResponse {
    pub vectors: Vec<MatchedVector>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatchedVector {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

// Service error response
#[derive(Clone, Debug, Deserialize)]
pub struct ServiceError {
    #[serde(rename = "__type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

fn default_true() -> bool {
    true
}