use std::cell::RefCell;
use std::rc::Rc;
use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tracing::info;

// Using all-MiniLM-L6-v2 for efficient embeddings (384 dimensions)
const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
const MODEL_REV: &str = "refs/pr/21";

thread_local! {
    static BERT_MODEL: RefCell<Option<Rc<BertModelWrapper>>> = RefCell::new(None);
}

pub struct BertModelWrapper {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BertModelWrapper {
    pub fn new(device: Device) -> Result<Self> {
        info!("Loading BERT model: {}", MODEL_ID);
        
        let repo = Repo::with_revision(MODEL_ID.into(), RepoType::Model, MODEL_REV.into());
        let api = Api::new()?;
        let api = api.repo(repo);
        
        let config_filename = api.get("config.json")?;
        let tokenizer_filename = api.get("tokenizer.json")?;
        let weights_filename = api.get("model.safetensors")?;
        
        // Load model configuration
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(anyhow::Error::msg)?;
        
        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)?
        };
        let model = BertModel::load(vb, &config)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }
    
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;
        
        let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;
        
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        
        // Apply mean pooling
        let pooled = self.mean_pooling(&embeddings)?;
        
        // L2 normalize
        let normalized = self.l2_normalize(&pooled)?;
        
        // Convert to Vec<f32>
        let embedding = normalized.squeeze(0)?.to_vec1::<f32>()?;
        
        Ok(embedding)
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
            let attention_mask = Tensor::new(attention_mask_vec, &self.device)?
                .reshape((batch_size, max_len))?;
            let token_type_ids = token_ids.zeros_like()?;
            
            let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
            
            // Apply mean pooling with attention mask
            let pooled = self.mean_pooling_with_mask(&embeddings, &attention_mask)?;
            
            // L2 normalize
            let normalized = self.l2_normalize(&pooled)?;
            
            // Extract individual embeddings
            for i in 0..batch_size {
                let embedding = normalized.get(i)?.to_vec1::<f32>()?;
                all_embeddings.push(embedding);
            }
        }
        
        Ok(all_embeddings)
    }
    
    fn mean_pooling(&self, embeddings: &Tensor) -> Result<Tensor> {
        let (_batch_size, seq_len, _hidden_size) = embeddings.dims3()?;
        let pooled = (embeddings.sum(1)? / (seq_len as f64))?;
        Ok(pooled)
    }
    
    fn mean_pooling_with_mask(&self, embeddings: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let expanded_mask = mask.unsqueeze(2)?;
        let masked_embeddings = embeddings.broadcast_mul(&expanded_mask)?;
        let sum_embeddings = masked_embeddings.sum(1)?;
        let sum_mask = expanded_mask.sum(1)?;
        let pooled = sum_embeddings.broadcast_div(&sum_mask)?;
        Ok(pooled)
    }
    
    fn l2_normalize(&self, embeddings: &Tensor) -> Result<Tensor> {
        let norm = embeddings
            .sqr()?
            .sum_keepdim(1)?
            .sqrt()?;
        let normalized = embeddings.broadcast_div(&norm)?;
        Ok(normalized)
    }
}

/// Get or create a thread-local instance of the BERT model
pub fn get_model() -> Result<Rc<BertModelWrapper>> {
    BERT_MODEL.with(|model| {
        let mut model_ref = model.borrow_mut();
        if model_ref.is_none() {
            info!("Initializing BERT model for thread {:?}", std::thread::current().id());
            let wrapper = BertModelWrapper::new(Device::Cpu)
                .context("Failed to create BERT model")?;
            *model_ref = Some(Rc::new(wrapper));
        }
        Ok(model_ref.as_ref().unwrap().clone())
    })
}

/// Embed a single text
pub fn embed_text(text: &str) -> Result<Vec<f32>> {
    let model = get_model()?;
    model.embed_text(text)
}

/// Embed multiple texts in batch
pub fn embed_texts(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let model = get_model()?;
    model.embed_batch(texts)
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