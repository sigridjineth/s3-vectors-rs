use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Sender};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::{
    batch_put_vectors, create_bucket_and_index,
    document::{Document, DocumentChunk, DocumentProcessor},
    embeddings,
    types::*,
    S3VectorsClient, Vector, VectorData,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub bucket_name: String,
    pub index_name: String,
    pub embedding_batch_size: usize,
    pub vector_upload_batch_size: usize,
    pub max_concurrent_embeddings: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            bucket_name: "rag-vectors-default".to_string(),
            index_name: "documents-default".to_string(),
            embedding_batch_size: 32,
            vector_upload_batch_size: 100,
            max_concurrent_embeddings: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSearchResult {
    pub chunk_id: String,
    pub content: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

pub struct RagPipeline {
    config: RagConfig,
    client: S3VectorsClient,
    document_processor: DocumentProcessor,
}

impl RagPipeline {
    pub fn new(config: RagConfig, client: S3VectorsClient) -> Self {
        let document_processor = DocumentProcessor::with_default_config();
        
        Self {
            config,
            client,
            document_processor,
        }
    }
    
    /// Initialize the S3 Vectors bucket and index
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing RAG pipeline with bucket: {} and index: {}", 
              self.config.bucket_name, self.config.index_name);
        
        create_bucket_and_index(
            &self.client,
            &self.config.bucket_name,
            &self.config.index_name,
            embeddings::embedding_dimensions(),
            DistanceMetric::Cosine,
        )
        .await
        .context("Failed to create bucket and index")?;
        
        Ok(())
    }
    
    /// Ingest documents from a directory
    pub async fn ingest_documents(&self, dir_path: &Path) -> Result<()> {
        let start_time = Instant::now();
        info!("Starting document ingestion from: {}", dir_path.display());
        
        // Process all documents in the directory
        let documents: Vec<Document> = self.document_processor
            .process_directory(dir_path)
            .await?;
        
        if documents.is_empty() {
            warn!("No documents found in directory");
            return Ok(());
        }
        
        info!("Found {} documents to process", documents.len());
        
        // Process documents in parallel using channels
        let (sender, receiver) = unbounded::<(DocumentChunk, Vec<f32>)>();
        
        // Spawn a task to handle vector uploads
        let bucket_name = self.config.bucket_name.clone();
        let index_name = self.config.index_name.clone();
        let batch_size = self.config.vector_upload_batch_size;
        let client = self.client.clone();
        
        let upload_handle = tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut total_uploaded = 0;
            let mut total_chunks = 0;
            let mut first_error = None;
            
            while let Ok((chunk, embedding)) = receiver.recv() {
                total_chunks += 1;
                let vector = Vector {
                    key: chunk.id.clone(),
                    data: VectorData {
                        float32: embedding,
                    },
                    metadata: Some(chunk.metadata),
                };
                
                buffer.push(vector);
                
                if buffer.len() >= batch_size {
                    match batch_put_vectors(&client, &bucket_name, &index_name, buffer.clone(), embeddings::embedding_dimensions()).await {
                        Ok(_) => {
                            total_uploaded += buffer.len();
                            debug!("Uploaded batch of {} vectors", buffer.len());
                        }
                        Err(e) => {
                            tracing::error!("Error uploading vectors: {}", e);
                            if first_error.is_none() {
                                first_error = Some(e.to_string());
                            }
                        }
                    }
                    buffer.clear();
                }
            }
            
            // Upload remaining vectors
            if !buffer.is_empty() {
                match batch_put_vectors(&client, &bucket_name, &index_name, buffer.clone(), embeddings::embedding_dimensions()).await {
                    Ok(_) => {
                        total_uploaded += buffer.len();
                        debug!("Uploaded final batch of {} vectors", buffer.len());
                    }
                    Err(e) => {
                        tracing::error!("Error uploading final batch: {}", e);
                        if first_error.is_none() {
                            first_error = Some(e.to_string());
                        }
                    }
                }
            }
            
            info!("Total vectors uploaded: {} out of {}", total_uploaded, total_chunks);
            
            if let Some(error) = first_error {
                if total_uploaded == 0 {
                    Err(anyhow::anyhow!("Failed to upload any vectors: {}", error))
                } else {
                    Err(anyhow::anyhow!("Partial upload: {} of {} vectors uploaded. First error: {}", 
                        total_uploaded, total_chunks, error))
                }
            } else {
                Ok(total_uploaded)
            }
        });
        
        // Process documents and generate embeddings in parallel
        let semaphore = std::sync::Arc::new(Semaphore::new(self.config.max_concurrent_embeddings));
        
        documents.par_iter().for_each(|document| {
            match self.process_document(document, &sender, &semaphore) {
                Ok(chunks_processed) => {
                    debug!("Processed {} chunks from document: {}", chunks_processed, document.id);
                }
                Err(e) => {
                    tracing::error!("Error processing document {}: {:?}", document.id, e);
                }
            }
        });
        
        // Close the channel
        drop(sender);
        
        // Wait for upload to complete
        let upload_result = upload_handle.await
            .context("Upload task panicked")?;
        
        let elapsed = start_time.elapsed();
        
        match upload_result {
            Ok(count) => {
                info!("Document ingestion completed in {:?}. Uploaded {} vectors.", elapsed, count);
                Ok(())
            },
            Err(e) => {
                tracing::error!("Document ingestion failed: {}", e);
                Err(e)
            }
        }
    }
    
    /// Process a single document
    fn process_document(
        &self,
        document: &Document,
        sender: &Sender<(DocumentChunk, Vec<f32>)>,
        semaphore: &std::sync::Arc<Semaphore>,
    ) -> Result<usize> {
        // Split document into chunks
        let chunks = self.document_processor.chunk_document(document)?;
        let chunk_count = chunks.len();
        
        // Process chunks in batches
        for batch in chunks.chunks(self.config.embedding_batch_size) {
            // Acquire semaphore permit
            let permit = semaphore.try_acquire();
            if permit.is_err() {
                // If no permit available, process synchronously
                let texts: Vec<&str> = batch.iter().map(|c| c.content.as_str()).collect();
                let embeddings = embeddings::embed_texts(&texts)?;
                
                for (chunk, embedding) in batch.iter().zip(embeddings.iter()) {
                    sender.send((chunk.clone(), embedding.clone()))?;
                }
            } else {
                // Process with permit
                let texts: Vec<&str> = batch.iter().map(|c| c.content.as_str()).collect();
                let embeddings = embeddings::embed_texts(&texts)?;
                
                for (chunk, embedding) in batch.iter().zip(embeddings.iter()) {
                    sender.send((chunk.clone(), embedding.clone()))?;
                }
            }
        }
        
        Ok(chunk_count)
    }
    
    /// Search for relevant documents
    pub async fn search(
        &self,
        query: &str,
        top_k: u32,
        filter: Option<serde_json::Value>,
    ) -> Result<Vec<RagSearchResult>> {
        info!("Searching for: {}", query);
        
        // Generate embedding for query
        let query_embedding = embeddings::embed_text(query)
            .context("Failed to embed query")?;
        
        // Create query request
        let query_request = QueryVectorsRequest {
            vector_bucket_name: self.config.bucket_name.clone(),
            index_name: self.config.index_name.clone(),
            query_vector: QueryVector {
                float32: query_embedding,
            },
            top_k,
            filter,
            return_metadata: true,
            return_distance: true,
        };
        
        // Execute query
        let response = self.client
            .query_vectors(query_request)
            .await
            .context("Failed to query vectors")?;
        
        // Convert results
        let results: Vec<RagSearchResult> = response
            .vectors
            .into_iter()
            .map(|matched| {
                let score = matched.distance.map(|d| 1.0 - d).unwrap_or(0.0);
                
                // Extract content from metadata
                let content = matched
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                
                RagSearchResult {
                    chunk_id: matched.key,
                    content,
                    score,
                    metadata: matched.metadata.unwrap_or_default(),
                }
            })
            .collect();
        
        info!("Found {} relevant documents", results.len());
        Ok(results)
    }
    
    /// Generate a response using retrieved context
    pub async fn generate_response(
        &self,
        query: &str,
        context_docs: &[RagSearchResult],
    ) -> Result<String> {
        // Build context from retrieved documents
        let context = context_docs
            .iter()
            .enumerate()
            .map(|(i, doc)| {
                format!("[Document {}]\n{}\n", i + 1, doc.content)
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        // In a real implementation, this would call an LLM
        // For demo purposes, we'll return a formatted response
        let response = format!(
            "Based on the retrieved context, here's a response to your query:\n\n\
            Query: {}\n\n\
            Context Summary:\n{}\n\n\
            [Note: In a production system, this would use an LLM to generate a proper response \
            based on the retrieved context.]",
            query, context
        );
        
        Ok(response)
    }
}

/// High-level RAG query function
pub async fn rag_query(
    pipeline: &RagPipeline,
    query: &str,
    top_k: u32,
) -> Result<String> {
    // Search for relevant documents
    let results = pipeline.search(query, top_k, None).await?;
    
    if results.is_empty() {
        return Ok("No relevant documents found for your query.".to_string());
    }
    
    // Generate response
    let response = pipeline.generate_response(query, &results).await?;
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rag_config() {
        let config = RagConfig::default();
        assert_eq!(config.bucket_name, "rag-vectors-default");
        assert_eq!(config.index_name, "documents-default");
        assert_eq!(config.embedding_batch_size, 32);
    }
}