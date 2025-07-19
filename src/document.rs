use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub path: String,
    pub content: String,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub file_type: String,
    pub size_bytes: usize,
    pub chunk_index: Option<usize>,
    pub total_chunks: Option<usize>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub content: String,
    pub chunk_index: usize,
    pub metadata: serde_json::Value,
}

pub struct ChunkingConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub min_chunk_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,     // tokens
            chunk_overlap: 50,   // tokens
            min_chunk_size: 100, // tokens
        }
    }
}

pub struct DocumentProcessor {
    config: ChunkingConfig,
    processed_count: AtomicUsize,
}

impl DocumentProcessor {
    pub fn new(config: ChunkingConfig) -> Self {
        Self {
            config,
            processed_count: AtomicUsize::new(0),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(ChunkingConfig::default())
    }

    /// Process a single document file
    pub async fn process_file(&self, path: &Path) -> Result<Document> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read file")?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let file_type = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_string();

        let metadata = DocumentMetadata {
            title: Some(file_name.to_string()),
            file_type,
            size_bytes: content.len(),
            chunk_index: None,
            total_chunks: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let doc_id = format!(
            "doc-{}",
            self.processed_count.fetch_add(1, Ordering::SeqCst)
        );

        Ok(Document {
            id: doc_id,
            path: path.to_string_lossy().to_string(),
            content,
            metadata,
        })
    }

    /// Split document into chunks
    pub fn chunk_document(&self, document: &Document) -> Result<Vec<DocumentChunk>> {
        let chunks = self.split_text_into_chunks(&document.content);
        let total_chunks = chunks.len();

        let mut document_chunks = Vec::new();

        for (index, chunk_content) in chunks.into_iter().enumerate() {
            // Truncate content for metadata to avoid exceeding S3 Vectors limit
            let content_preview = if chunk_content.len() > 500 {
                // Find a safe UTF-8 boundary at or before 500 bytes
                let mut truncate_pos = 500;
                while truncate_pos > 0 && !chunk_content.is_char_boundary(truncate_pos) {
                    truncate_pos -= 1;
                }
                format!("{}...", &chunk_content[..truncate_pos])
            } else {
                chunk_content.clone()
            };

            let chunk_metadata = serde_json::json!({
                "document_id": document.id,
                "document_path": document.path,
                "title": document.metadata.title,
                "file_type": document.metadata.file_type,
                "chunk_index": index,
                "total_chunks": total_chunks,
                "created_at": document.metadata.created_at,
                "content": content_preview,
            });

            let chunk = DocumentChunk {
                id: format!("{}-chunk-{}", document.id, index),
                document_id: document.id.clone(),
                content: chunk_content,
                chunk_index: index,
                metadata: chunk_metadata,
            };

            document_chunks.push(chunk);
        }

        info!(
            "Split document {} into {} chunks",
            document.id, total_chunks
        );

        Ok(document_chunks)
    }

    /// Split text into overlapping chunks
    fn split_text_into_chunks(&self, text: &str) -> Vec<String> {
        // Simple word-based chunking
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.len() <= self.config.chunk_size {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < words.len() {
            let end = (start + self.config.chunk_size).min(words.len());
            let chunk = words[start..end].join(" ");

            if chunk.split_whitespace().count() >= self.config.min_chunk_size {
                chunks.push(chunk);
            }

            start += self.config.chunk_size - self.config.chunk_overlap;

            if start + self.config.min_chunk_size > words.len() {
                break;
            }
        }

        chunks
    }

    /// Process multiple files in parallel
    pub async fn process_directory(&self, dir_path: &Path) -> Result<Vec<Document>> {
        let mut documents = Vec::new();
        let mut entries = tokio::fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("txt") | Some("md") => match self.process_file(&path).await {
                        Ok(doc) => {
                            debug!("Processed document: {}", path.display());
                            documents.push(doc);
                        }
                        Err(e) => {
                            tracing::error!("Error processing {}: {}", path.display(), e);
                        }
                    },
                    _ => {
                        debug!("Skipping non-text file: {}", path.display());
                    }
                }
            }
        }

        info!("Processed {} documents from directory", documents.len());
        Ok(documents)
    }
}

/// Extract title from markdown or text content
pub fn extract_title(content: &str) -> Option<String> {
    // Try to extract markdown title
    let title_regex = Regex::new(r"^#\s+(.+)$").ok()?;
    for line in content.lines() {
        if let Some(captures) = title_regex.captures(line) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Otherwise, use first non-empty line
    content
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

lazy_static! {
    static ref WHITESPACE_REGEX: Regex =
        Regex::new(r"\s+").expect("Failed to compile whitespace regex");
    static ref SPECIAL_REGEX: Regex =
        Regex::new(r#"[^\w\s\.\,\!\?\-\'\"]"#).expect("Failed to compile special characters regex");
}

/// Clean and normalize text content
pub fn clean_text(text: &str) -> String {
    // Remove multiple whitespaces
    let cleaned = WHITESPACE_REGEX.replace_all(text, " ");

    // Remove special characters that might interfere with embedding
    let cleaned = SPECIAL_REGEX.replace_all(&cleaned, " ");

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_chunking() {
        let processor = DocumentProcessor::with_default_config();
        let text = (0..1000)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");

        let chunks = processor.split_text_into_chunks(&text);

        // Verify chunks are created
        assert!(chunks.len() > 1);

        // Verify overlap
        let first_chunk_words: Vec<&str> = chunks[0].split_whitespace().collect();
        let second_chunk_words: Vec<&str> = chunks[1].split_whitespace().collect();

        let overlap_start = first_chunk_words.len() - 50;
        for i in 0..50 {
            assert_eq!(first_chunk_words[overlap_start + i], second_chunk_words[i]);
        }
    }
}
