# S3 Vectors Rust SDK with RAG Demo

A comprehensive Rust SDK for Amazon S3 Vectors with a complete RAG (Retrieval-Augmented Generation) implementation using Candle for embeddings.

## Usage

```rust
use s3_vectors::{QueryVector, QueryVectorsRequest, S3VectorsClient, Vector};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = S3VectorsClient::new("us-east-1");
    
    // Create bucket
    client.create_vector_bucket("my-vectors").await?;
    
    // Create index
    let index_request = CreateIndexRequest {
        bucket_name: "my-vectors".to_string(),
        index_name: "products".to_string(),
        vector_dimensions: 1024,
        distance_metric: "Cosine".to_string(),
    };
    client.create_index(index_request).await?;
    
    // Put vectors
    let request = PutVectorsRequest {
        bucket_name: "my-vectors".to_string(),
        index_name: "products".to_string(),
        vectors: vec![
            Vector {
                key: "vec-1".to_string(),
                vector: vec![0.1; 1024],
                metadata: Some(json!({"category": "electronics"})),
            },
        ],
    };
    client.put_vectors(request).await?;
    
    // Query vectors
    let query = QueryVectorsRequest {
        bucket_name: "my-vectors".to_string(),
        index_name: "products".to_string(),
        query_vector: QueryVector {
            float32: vec![0.1; 1024],
        },
        top_k: 10,
        filter: None,
        return_metadata: true,
        return_distance: true,
    };
    let results = client.query_vectors(query).await?;
    
    Ok(())
}
```

## Configuration

Set environment variables:
- `AWS_REGION` (default: us-east-1)
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_SESSION_TOKEN` (optional)

## RAG Demo

This project includes a complete RAG implementation that:
- Uses Candle framework for BERT embeddings (all-MiniLM-L6-v2 model)
- Processes documents in parallel using Rayon
- Stores vectors in Amazon S3 Vectors
- Provides semantic search capabilities

### Running the RAG Demo

1. **Build the project**:
```bash
cargo build --release --example rag_demo
```

2. **Initialize the RAG pipeline** (creates S3 Vectors bucket and index):
```bash
cargo run --example rag_demo -- init
```

3. **Ingest documents**:
```bash
cargo run --example rag_demo -- ingest --directory test_documents
```

4. **Query the system**:
```bash
cargo run --example rag_demo -- query --query "What is RAG?" --top-k 5
```

5. **Interactive mode**:
```bash
cargo run --example rag_demo -- interactive
```

### Architecture

The RAG implementation consists of:

- **Embeddings Module** (`src/embeddings.rs`): BERT model wrapper using Candle
- **Document Processor** (`src/document.rs`): Text chunking and metadata extraction
- **RAG Pipeline** (`src/rag.rs`): Complete ingestion and query workflow
- **S3 Vectors Integration**: Native support for Amazon S3 Vectors storage

### Features

- **High Performance**: Parallel document processing with Rayon
- **Efficient Embeddings**: Thread-local model caching for optimal performance
- **Smart Chunking**: Configurable chunk size with overlap
- **Metadata Support**: Rich metadata filtering capabilities
- **Cost Optimized**: Leverages S3 Vectors for economical vector storage

### Performance

On a 10-core machine, the system can:
- Process and embed ~25,000 words per second
- Ingest documents with parallel embedding generation
- Query with sub-second latency using S3 Vectors

### Example Code

```rust
use s3_vectors::{
    rag::{RagConfig, RagPipeline},
    S3VectorsClient,
};

// Create client and pipeline
let client = S3VectorsClient::from_env()?;
let config = RagConfig::default();
let pipeline = RagPipeline::new(config, client);

// Initialize
pipeline.initialize().await?;

// Ingest documents
pipeline.ingest_documents(&PathBuf::from("docs")).await?;

// Query
let results = pipeline.search("What is vector search?", 5, None).await?;
```