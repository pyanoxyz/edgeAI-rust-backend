// use rust_bert::distilbert::{DistilBertConfig, DistilBertModelMaskedLM};
use rust_bert::bert::{BertConfig, BertForMaskedLM};

use rust_bert::resources::{RemoteResource, ResourceProvider, LocalResource};
use rust_bert::Config;
use rust_tokenizers::tokenizer::{BertTokenizer, Tokenizer, TruncationStrategy};
use tch::{nn, Device, Tensor, no_grad};
use std::path::Path;
use std::fs;
use anyhow::Result;
use std::env;
use tokio::task;
use log::{debug,info};
use std::fs::create_dir_all;
use tch::IndexOp;

/// Directory to save the model
const ROOT_PYANO_DIR: &str = ".pyano";
const PYANO_MODELS_DIR: &str = ".pyano/models";

pub struct AttentionCalculator {
    model: BertForMaskedLM,
    tokenizer: BertTokenizer,
    device: Device,
}

impl AttentionCalculator {
    pub fn new(model_path: &str) -> Result<Self> {
        // Load the pre-trained DistilBERT model and tokenizer
        let device = Device::cuda_if_available();

        let config_path = format!("{}/bert-bert-uncased/config.json", model_path);
        let vocab_path = format!("{}/bert-bert-uncased/vocab.txt", model_path);
        let weights_path = format!("{}/bert-bert-uncased/model.ot", model_path);

        // Load DistilBERT config
        let config = BertConfig::from_file(config_path);

        // Load DistilBERT model
        let mut vs = nn::VarStore::new(device);
        let model = BertForMaskedLM::new(&vs.root(), &config);

        // Load tokenizer
        let tokenizer = BertTokenizer::from_file(vocab_path.clone(), true, true)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer from {}: {:?}", vocab_path, e))?;
    
        // Load model weights
        vs.load(weights_path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to load model weights from {}: {:?}", weights_path, e))?;
    
        Ok(AttentionCalculator { model, tokenizer, device })
    }

    pub fn calculate_attention_scores(&self, prompt: &str, threshold: f32) -> Result<(Vec<String>, Vec<f32>)> {
        // Tokenize input text into chunks
        let tokenized_input = self.tokenizer.encode(prompt, None, 512, &TruncationStrategy::LongestFirst, 0);

          // Extract token_ids from tokenized_input
        let token_ids = &tokenized_input.token_ids;
        info!("Total length of tokens {}", token_ids.len());        
        let input_ids = Tensor::from_slice(&tokenized_input.token_ids).unsqueeze(0).to(self.device);

        // Forward pass through the model to get attention scores
        let outputs = no_grad(|| {
            self.model
            .forward_t(Some(&input_ids), None, None, None, None, None, None, false) // Only the last argument is a boolean
        });

        let mut attention_scores: Vec<Vec<f32>> = Vec::new();
        let mut self_attention_scores: Vec<f32> = Vec::new(); 
        if let Some(attentions) = outputs.all_attentions {
            if let Some(last_attention) = attentions.last() {
                let attention = last_attention.copy();
        
                // Average across attention heads only, keeping the sequence intact
                let attention_weights = attention.mean_dim(&[1i64][..], false, tch::Kind::Float);
                
                // Check the dimensions of the attention weights tensor
                
                // 2D attention scores for each token with respect to other tokens
                let attention_scores_2d = attention_weights.squeeze().to_kind(tch::Kind::Float).contiguous();

                let num_tokens = attention_scores_2d.size()[0]; // Ensure you're indexing correctly here
                self_attention_scores = Vec::with_capacity(num_tokens as usize);
        
                for i in 0..num_tokens {
                    let self_attention = attention_scores_2d.i((i, i)).double_value(&[]);
                    self_attention_scores.push(self_attention as f32);  // Self-attention of token i
                }
        
                // info!("Self-attention scores for all tokens: {:?}", self_attention_scores);
        
                // Map tokens back to their original words
                // Decode tokens back to subword units instead of full strings
                let tokens: Vec<String> = token_ids
                .iter()
                .map(|id| self.tokenizer.decode(&[*id], true, true))
                .collect();

                // Filter out special tokens ([CLS], [SEP], [PAD]) and scores below threshold
                let valid_tokens_attention: Vec<(String, f32)> = tokens
                .into_iter()
                .zip(self_attention_scores.into_iter())
                .filter(|(token, score)| {
                    !["[CLS]", "[SEP]", "[PAD]"].contains(&token.as_str()) 
                    && !token.starts_with("##")
                    && *score > threshold
                })
                .collect();

                info!("Total length of tokens execeeding threshold {}", valid_tokens_attention.len());        

                // Log the tokens and their attention
                let (valid_tokens, valid_attention_scores): (Vec<_>, Vec<_>) = valid_tokens_attention.into_iter().unzip();
                return Ok((valid_tokens, valid_attention_scores));

            } else {
                return Err(anyhow::anyhow!("No last attention scores found"));
            }
        } else {
            return Err(anyhow::anyhow!("No attention output available from the model"));
        }
  
    }
}


fn download_and_save_model(save_path: &str) -> Result<()> {
    // Define paths for config, vocab, and weights
    let config_dest = format!("{}/bert-bert-uncased/config.json", save_path);
    let vocab_dest = format!("{}/bert-bert-uncased/vocab.txt", save_path);
    let weights_dest = format!("{}/bert-bert-uncased/model.ot", save_path);

    // Check if any of the model files (config, vocab, weights) already exist
    if Path::new(&config_dest).exists() && Path::new(&vocab_dest).exists() && Path::new(&weights_dest).exists() {
        debug!("Prompt compression model files already exists at {}. Skipping download.", save_path);
        return Ok(());
    }

    // Create the directory if it doesn't exist
    let distillbert_dir = format!("{}/bert-bert-uncased", save_path);
    if !Path::new(&distillbert_dir).exists() {
        create_dir_all(&distillbert_dir)?;
        debug!("Created directory: {}", distillbert_dir);
    }

    // Downloading config, vocab, and weights only if they don't exist
    if !Path::new(&config_dest).exists() {
        let config_resource = RemoteResource::from_pretrained(rust_bert::bert::BertConfigResources::BERT);
        let config_path = config_resource.get_local_path()?;
        fs::copy(config_path, &config_dest)?;
        debug!("Config saved to {}", config_dest);
    } else {
        debug!("Config already exists at {}. Skipping.", config_dest);
    }

    if !Path::new(&vocab_dest).exists() {
        let vocab_resource = RemoteResource::from_pretrained(rust_bert::bert::BertVocabResources::BERT);
        let vocab_path = vocab_resource.get_local_path()?;
        fs::copy(vocab_path, &vocab_dest)?;
        debug!("Vocab saved to {}", vocab_dest);
    } else {
        debug!("Vocab already exists at {}. Skipping.", vocab_dest);
    }

    if !Path::new(&weights_dest).exists() {
        let weights_resource = RemoteResource::from_pretrained(rust_bert::bert::BertModelResources::BERT);
        let weights_path = weights_resource.get_local_path()?;
        fs::copy(weights_path, &weights_dest)?;
        debug!("Weights saved to {}", weights_dest);
    } else {
        debug!("Weights already exist at {}. Skipping.", weights_dest);
    }

    debug!("Model successfully saved to {}", save_path);
    Ok(())
}


pub async fn get_attention_scores(text: &str) -> Result<(Vec<String>, Vec<f32>)> {
    let home_dir = env::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
    let pyano_models_dir = home_dir.join(".pyano/models");

    // Ensure the model directory exists
    fs::create_dir_all(&pyano_models_dir)?;

    // Ensure the model is downloaded
    let pyano_models_dir_str = pyano_models_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?
        .to_string();

    // Clone variables for the closure
    let text_clone = text.to_string();
    let pyano_models_dir_str_clone = pyano_models_dir_str.clone();

    // Run the download_and_save_model in a blocking task
    let (tokens, attention_scores): (Vec<String>, Vec<f32>) = task::spawn_blocking(move || -> Result<(Vec<String>, Vec<f32>), anyhow::Error> {
        // Download and save the model, handle any errors
        download_and_save_model(&pyano_models_dir_str_clone).map_err(|e| anyhow::anyhow!(e))?;

        // Create the AttentionCalculator
        let attention_calculator = AttentionCalculator::new(&pyano_models_dir_str_clone)?;

        // Calculate attention scores for the input text
        let (tokens, attention_scores) = attention_calculator.calculate_attention_scores(&text_clone, 0.06)?;

        Ok((tokens, attention_scores))
    })
    .await??;

    Ok((tokens, attention_scores))
}