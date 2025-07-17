use std::rc::Rc;
use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tracing::{debug, info};

// Using all-MiniLM-L6-v2 for efficient embeddings (384 dimensions)
const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
const MODEL_REV: &str = "main";

thread_local! {
    static BERT_MODEL: Rc<BertModelWrapper> = {
        info!("Loading BERT model on thread: {:?}", std::thread::current().id());
        match BertModelWrapper::new(Device::Cpu) {
            Ok(model) => Rc::new(model),
            Err(e) => panic!("Failed to load BERT model: {}", e),
        }
    };
}

pub struct BertModelWrapper {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BertModelWrapper {
    pub fn new(device: Device) -> Result<Self> {
        info!("Loading BERT model: {} (revision: {})", MODEL_ID, MODEL_REV);
        
        // Try to load from local files first
        let model_dir = std::path::Path::new("models/all-MiniLM-L6-v2");
        let config_filename = model_dir.join("config.json");
        let tokenizer_filename = model_dir.join("tokenizer.json");
        let weights_filename = model_dir.join("model.safetensors");
        
        // Check if local files exist
        if config_filename.exists() && tokenizer_filename.exists() && weights_filename.exists() {
            info!("Loading model from local files");
            return Self::load_from_files(config_filename, tokenizer_filename, weights_filename, device);
        }
        
        // Download from HuggingFace
        info!("Model files not found locally, downloading from HuggingFace...");
        let repo = Repo::with_revision(MODEL_ID.into(), RepoType::Model, MODEL_REV.into());
        let api = Api::new()
            .context("Failed to create HuggingFace API client")?;
        let api = api.repo(repo);
        
        let config_filename = api.get("config.json")
            .context("Failed to download config.json from HuggingFace")?;
        let tokenizer_filename = api.get("tokenizer.json")
            .context("Failed to download tokenizer.json from HuggingFace")?;
        let weights_filename = api.get("model.safetensors")
            .context("Failed to download model.safetensors from HuggingFace")?;
        
        Self::load_from_files(config_filename, tokenizer_filename, weights_filename, device)
    }
    
    fn load_from_files(
        config_filename: impl AsRef<std::path::Path>,
        tokenizer_filename: impl AsRef<std::path::Path>,
        weights_filename: impl AsRef<std::path::Path>,
        device: Device,
    ) -> Result<Self> {
        let config_filename = config_filename.as_ref();
        let tokenizer_filename = tokenizer_filename.as_ref();
        let weights_filename = weights_filename.as_ref();
        
        // Load model configuration
        let config = std::fs::read_to_string(&config_filename)
            .with_context(|| format!("Failed to read config file: {:?}", config_filename))?;
        let config: Config = serde_json::from_str(&config)
            .context("Failed to parse model config.json")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_filename)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("Failed to load tokenizer from: {:?}", tokenizer_filename))?;
        
        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename.to_path_buf()], DTYPE, &device)
                .with_context(|| format!("Failed to load model weights from: {:?}", weights_filename))?
        };
        let model = BertModel::load(vb, &config)
            .context("Failed to load BERT model from weights")?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }
    
    pub fn embed_sentence(&self, sentence: &str) -> Result<Tensor> {
        let tokens = self
            .tokenizer
            .encode(sentence, true)
            .map_err(anyhow::Error::msg)?;
        let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;
        
        let start = std::time::Instant::now();
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        debug!("Time taken for forward: {:?}", start.elapsed());
        debug!("Embeddings shape: {:?}", embeddings.dims());
        
        // Apply max pooling for single sentences (as per reference)
        let embeddings = Self::apply_max_pooling(&embeddings)?;
        debug!("Embeddings after pooling: {:?}", embeddings.dims());
        
        // L2 normalize
        let embeddings = Self::l2_normalize(&embeddings)?;
        Ok(embeddings)
    }
    
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let embedding_tensor = self.embed_sentence(text)?;
        let embedding = embedding_tensor.squeeze(0)?.to_vec1::<f32>()?;
        Ok(embedding)
    }
    
    pub fn embed_sentences(&self, sentences: &[&str], apply_mean: bool) -> Result<Tensor> {
        let mut all_tokens = Vec::with_capacity(sentences.len());
        for sentence in sentences {
            let tokens = self
                .tokenizer
                .encode(*sentence, true)
                .map_err(anyhow::Error::msg)?;
            all_tokens.push(tokens);
        }

        let batch_size = sentences.len();
        let max_length = all_tokens.iter()
            .map(|t| t.get_ids().len())
            .max()
            .unwrap_or(0);

        let mut token_ids = Vec::with_capacity(batch_size * max_length);
        let mut attention_mask = Vec::with_capacity(batch_size * max_length);

        for tokens in all_tokens {
            let mut ids = tokens.get_ids().to_vec();
            let mut mask = tokens.get_attention_mask().to_vec();
            
            // Pad to max length
            ids.resize(max_length, 0);
            mask.resize(max_length, 0);
            
            token_ids.extend_from_slice(&ids);
            attention_mask.extend_from_slice(&mask);
        }

        let token_ids = Tensor::new(token_ids, &self.device)?.reshape((batch_size, max_length))?;
        let token_type_ids = token_ids.zeros_like()?;
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        let embeddings = Self::apply_mean_pooling(&embeddings)?;
        let embeddings = Self::l2_normalize(&embeddings)?;
        
        if apply_mean {
            let embeddings = Self::apply_mean_pooling(&embeddings)?;
            Ok(embeddings)
        } else {
            Ok(embeddings)
        }
    }
    
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::new();
        
        // Process in smaller batches to avoid memory issues
        for chunk in texts.chunks(32) {
            let mut batch_tokens = Vec::new();
            
            for text in chunk {
                let tokens = self
                    .tokenizer
                    .encode(*text, true)
                    .map_err(anyhow::Error::msg)?;
                batch_tokens.push(tokens);
            }
            
            // Pad sequences to same length
            let max_len = batch_tokens
                .iter()
                .map(|t| t.get_ids().len())
                .max()
                .unwrap_or(0);
            
            let mut token_ids_vec: Vec<u32> = Vec::new();
            let mut attention_mask_vec: Vec<u32> = Vec::new();
            
            for tokens in &batch_tokens {
                let mut ids = tokens.get_ids().to_vec();
                let mut mask = tokens.get_attention_mask().to_vec();
                
                // Pad to max length
                ids.resize(max_len, 0);
                mask.resize(max_len, 0);
                
                token_ids_vec.extend(&ids);
                attention_mask_vec.extend(&mask);
            }
            
            let batch_size = chunk.len();
            let token_ids = Tensor::new(token_ids_vec, &self.device)?
                .reshape((batch_size, max_len))?;
            let _attention_mask = Tensor::new(attention_mask_vec, &self.device)?
                .reshape((batch_size, max_len))?
                .to_dtype(candle_core::DType::F32)?;
            let token_type_ids = token_ids.zeros_like()?;
            
            let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
            
            // Apply mean pooling for batches
            let pooled = Self::apply_mean_pooling(&embeddings)?;
            
            // L2 normalize
            let normalized = Self::l2_normalize(&pooled)?;
            
            // Extract individual embeddings
            for i in 0..batch_size {
                let embedding = normalized.get(i)?.to_vec1::<f32>()?;
                all_embeddings.push(embedding);
            }
        }
        
        Ok(all_embeddings)
    }
    
    pub fn apply_max_pooling(embeddings: &Tensor) -> Result<Tensor> {
        Ok(embeddings.max(1)?)
    }
    
    /// Apply mean pooling to the embeddings
    /// The input tensor should either have the shape (n_sentences, n_tokens, hidden_size) or (n_tokens, hidden_size)
    /// depending on whether the input is a batch of sentences or a single sentence
    pub fn apply_mean_pooling(embeddings: &Tensor) -> Result<Tensor> {
        match embeddings.rank() {
            3 => {
                let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
                (embeddings.sum(1)? / (n_tokens as f64)).map_err(anyhow::Error::msg)
            }
            2 => {
                let (n_tokens, _hidden_size) = embeddings.dims2()?;
                (embeddings.sum(0)? / (n_tokens as f64)).map_err(anyhow::Error::msg)
            }
            _ => anyhow::bail!("Unsupported tensor rank for mean pooling"),
        }
    }
    
    pub fn l2_normalize(embeddings: &Tensor) -> Result<Tensor> {
        let normalized = embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?;
        Ok(normalized)
    }
}

/// Get the thread-local instance of the BERT model
pub fn get_model() -> Result<Rc<BertModelWrapper>> {
    BERT_MODEL.with(|model| Ok(model.clone()))
}

/// Embed a single text
pub fn embed_text(text: &str) -> Result<Vec<f32>> {
    let model = get_model()?;
    model.embed_text(text)
}

/// Embed multiple texts in batch
pub fn embed_texts(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let model = get_model()?;
    
    // Use embed_sentences without apply_mean to get individual embeddings
    let embeddings_tensor = model.embed_sentences(texts, false)?;
    
    // Convert tensor to Vec<Vec<f32>>
    let mut result = Vec::with_capacity(texts.len());
    for i in 0..texts.len() {
        let embedding = embeddings_tensor.get(i)?.to_vec1::<f32>()?;
        result.push(embedding);
    }
    
    Ok(result)
}

/// Get the dimension of embeddings produced by the model
pub fn embedding_dimensions() -> u32 {
    384 // all-MiniLM-L6-v2 produces 384-dimensional embeddings
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedding_dimensions() {
        assert_eq!(embedding_dimensions(), 384);
    }
    
    #[test]
    fn test_single_embedding() {
        let text = "This is a test sentence.";
        let embedding = embed_text(text).unwrap();
        assert_eq!(embedding.len(), 384);
        
        // Check that embedding is normalized (L2 norm ≈ 1)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_batch_embedding() {
        let texts = vec![
            "First test sentence.",
            "Second test sentence with more words.",
            "Third one.",
        ];
        let embeddings = embed_texts(&texts).unwrap();
        
        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 384);
        }
    }
}