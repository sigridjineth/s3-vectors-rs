use serde::{Deserialize, Serialize};
use serde_json::Value;

// Enums
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[derive(clap::ValueEnum)]
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
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Float32,
}

impl Default for DataType {
    fn default() -> Self {
        DataType::Float32
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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
    pub vector_bucket_name: String,
    pub vector_bucket_arn: String,
    pub creation_time: f64,
    #[serde(default)]
    pub status: Option<BucketStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_configuration: Option<EncryptionConfiguration>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sse_type: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVectorBucketRequest {
    pub vector_bucket_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVectorBucketResponse {
    // Empty response body for successful creation
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteVectorBucketRequest {
    pub vector_bucket_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorBucketsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub dimension: u32,
    pub data_type: DataType,
    pub distance_metric: DistanceMetric,
    pub creation_time: f64,
    #[serde(default)]
    pub status: Option<IndexStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIndexRequest {
    pub vector_bucket_name: String,
    pub index_name: String,
    pub dimension: u32,
    pub data_type: DataType,
    pub distance_metric: DistanceMetric,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_configuration: Option<MetadataConfiguration>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_filterable_metadata_keys: Option<Vec<String>>,
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
    pub vector_bucket_name: String,
    pub index_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeIndexRequest {
    pub vector_bucket_name: String,
    pub index_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndexesRequest {
    pub vector_bucket_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    pub index_name: String,
    pub index_arn: String,
    pub vector_bucket_name: String,
    pub creation_time: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndexesResponse {
    pub indexes: Vec<IndexSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetIndexRequest {
    pub vector_bucket_name: String,
    pub index_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetIndexResponse {
    pub index: IndexInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub dimension: u32,
    pub index_name: String,
    pub vector_bucket_name: String,
    pub created_at: String,
    pub status: IndexStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_count: Option<u64>,
}

// Vector types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorData {
    pub float32: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vector {
    pub key: String,
    pub data: VectorData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl Vector {
    pub fn validate(&self, expected_dimensions: u32) -> anyhow::Result<()> {
        if self.data.float32.len() != expected_dimensions as usize {
            anyhow::bail!(
                "Vector dimension mismatch: expected {}, got {}",
                expected_dimensions,
                self.data.float32.len()
            );
        }
        
        // Validate that vector values are not NaN or Infinity
        for (i, &value) in self.data.float32.iter().enumerate() {
            if value.is_nan() {
                anyhow::bail!("Vector contains NaN at index {}", i);
            }
            if value.is_infinite() {
                anyhow::bail!("Vector contains infinite value at index {}", i);
            }
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
    pub vector_bucket_name: String,
    pub index_name: String,
    pub vectors: Vec<Vector>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVectorsRequest {
    pub vector_bucket_name: String,
    pub index_name: String,
    pub keys: Vec<String>,
    #[serde(default = "default_true")]
    pub return_vector: bool,
    #[serde(default = "default_true")]
    pub return_metadata: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVectorsResponse {
    pub vectors: Vec<RetrievedVector>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub vector_bucket_name: String,
    pub index_name: String,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorsRequest {
    pub vector_bucket_name: String,
    pub index_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListVectorsResponse {
    pub keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryVectorsRequest {
    pub vector_bucket_name: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryVectorsResponse {
    pub vectors: Vec<MatchedVector>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub error_type: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

fn default_true() -> bool {
    true
}