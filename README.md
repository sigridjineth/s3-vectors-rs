# AWS S3 Vectors Rust CLI

Unofficial CLI for AWS S3 Vectors with RAG capabilities.

## Installation

```bash
cargo build --release

# Download ML models (required for RAG, ~90MB)
s3-vectors install-models
```

## Quick Start

```bash
# Interactive mode
s3-vectors

# Create bucket and index
s3-vectors bucket create my-vectors
s3-vectors index create my-vectors embeddings -d 384

# Add vectors
s3-vectors vector put my-vectors embeddings key1 -d "0.1,0.2,0.3..."

# Query similar vectors
s3-vectors vector query my-vectors embeddings -q "0.1,0.2,0.3..." -t 10
```

## Core Commands

### Bucket Operations
- `bucket create/list/get/delete <name>`

### Index Operations
- `index create <bucket> <name> -d <dimensions> [-m cosine|euclidean]`
- `index list/get/delete <bucket> <name>`

### Vector Operations
- `vector put <bucket> <index> <key> -d <data> [-m metadata]`
- `vector put <bucket> <index> -f <file>` (batch)
- `vector get <bucket> <index> <keys>`
- `vector query <bucket> <index> -q <vector> -t <top_k>`
- `vector delete <bucket> <index> <keys>`

### Policy Management
- `policy put/get/delete <bucket> [-f file | -p inline]`

## Interactive Mode

Run `s3-vectors` without arguments for REPL mode:
- Same commands without `s3-vectors` prefix
- Command history with arrows
- `help` or `?` for assistance
- `exit` or `quit` to leave

## Global Options

- `-r, --region <REGION>` (default: us-east-1)
- `-p, --profile <PROFILE>` (AWS profile)
- `-o, --output <FORMAT>` (json|table|yaml)
- `-v, --verbose` (detailed output)

## Key Limits

- Preview only: us-east-1, us-west-2
- Vector dimensions: 1-4096
- Batch size: 500 vectors max
- Distance metrics: euclidean, cosine
- This codebase only supports all-MiniLM-L6-v2 at the moment

## Environment Variables

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_REGION`
- `AWS_PROFILE`

## Demo
### RAGDemo
```
cargo build --release --example rag_demo
cargo run --example rag_demo -- init
cargo run --example rag_demo -- ingest --directory test_documents
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running `target/debug/examples/rag_demo ingest --directory test_documents`
📄 Ingesting documents from: test_documents
2025-07-17T07:16:35.974147Z  INFO s3_vectors::rag: Starting document ingestion from: test_documents
2025-07-17T07:16:35.975614Z  INFO s3_vectors::document: Processed 2 documents from directory
2025-07-17T07:16:35.975647Z  INFO s3_vectors::rag: Found 2 documents to process
2025-07-17T07:16:35.982392Z  INFO s3_vectors::document: Split document doc-1 into 20 chunks
2025-07-17T07:16:35.982435Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(33)
2025-07-17T07:16:35.982456Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:16:35.982475Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:16:35.982631Z  INFO s3_vectors::document: Split document doc-0 into 23 chunks
2025-07-17T07:16:35.982646Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(31)
2025-07-17T07:16:35.982652Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:16:35.982663Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:17:23.901052Z  INFO s3_vectors::deploy: Putting 43 vectors to index documents-sigrid in bucket rag-demo-sigrid
2025-07-17T07:17:25.407469Z  INFO s3_vectors::deploy: Successfully put 43 vectors
2025-07-17T07:17:25.407660Z  INFO s3_vectors::rag: Total vectors uploaded: 43
2025-07-17T07:17:25.407905Z  INFO s3_vectors::rag: Document ingestion completed in 49.55480375s
✅ Document ingestion completed in 49.55486025s

cargo run --example rag_demo -- query --query "what is AI?" --top-k 5
2025-07-17T07:45:54.549425Z  INFO s3_vectors::rag: Searching for: hat is AI
2025-07-17T07:45:54.549536Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(1)
2025-07-17T07:45:54.549554Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:45:54.549626Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:45:56.634915Z  INFO s3_vectors::deploy: Querying vectors in index documents-sigrid of bucket rag-demo-sigrid
2025-07-17T07:45:57.386262Z  INFO s3_vectors::rag: Found 5 relevant documents
```