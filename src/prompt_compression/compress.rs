use rust_bert::distilbert::{DistilBertConfig, DistilBertModelMaskedLM};
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

/// Directory to save the model
const ROOT_PYANO_DIR: &str = ".pyano";
const PYANO_MODELS_DIR: &str = ".pyano/models";

pub struct AttentionCalculator {
    model: DistilBertModelMaskedLM,
    tokenizer: BertTokenizer,
    device: Device,
}

impl AttentionCalculator {
    pub fn new(model_path: &str) -> Result<Self> {
        // Load the pre-trained DistilBERT model and tokenizer
        let device = Device::cuda_if_available();

        let config_path = format!("{}/distillbert//config.json", model_path);
        let vocab_path = format!("{}/distillbert//vocab.txt", model_path);
        let weights_path = format!("{}/distillbert//model.ot", model_path);

        // Load DistilBERT config
        let config = DistilBertConfig::from_file(config_path);

        // Load DistilBERT model
        let mut vs = nn::VarStore::new(device);
        let model = DistilBertModelMaskedLM::new(&vs.root(), &config);

        // Load tokenizer
        let tokenizer = BertTokenizer::from_file(vocab_path.clone(), true, true)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer from {}: {:?}", vocab_path, e))?;
    
        // Load model weights
        vs.load(weights_path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to load model weights from {}: {:?}", weights_path, e))?;
    
    
        Ok(AttentionCalculator { model, tokenizer, device })
    }

    pub fn calculate_attention_scores(&self, prompt: &str) -> Result<(Vec<String>, Vec<f32>)> {
        // Tokenize input text into chunks
        let tokenized_input = self.tokenizer.encode(prompt, None, 512, &TruncationStrategy::LongestFirst, 0);
        let input_ids = Tensor::from_slice(&tokenized_input.token_ids).unsqueeze(0).to(self.device);

        // Forward pass through the model to get attention scores
        let outputs = no_grad(|| {
            self.model
                .forward_t(Some(&input_ids), None, None, false)
        })?;

        let attention_scores: Vec<f32>;

        if let Some(attentions) = outputs.all_attentions {
            if let Some(last_attention) = attentions.last() {
                let attention = last_attention.copy();
        
                // Continue processing attention, such as averaging attention across all heads
                let attention_weights = attention.mean_dim(&[1i64][..], false, tch::Kind::Float);  // Convert array to slice
                // Now, attention_weights can be used further...
                attention_scores = Vec::<f32>::try_from(attention_weights.squeeze().to_kind(tch::Kind::Float).contiguous())?;

            } else {
                return Err(anyhow::anyhow!("No last attention scores found"));
            }
        } else {
            return Err(anyhow::anyhow!("No attention output available from the model"));
        }

        // let attention = outputs.all_attentions.unwrap().last().unwrap().copy();
        
        // // Average attention across all heads
        // let attention_weights = attention.mean_dim(&[1i64][..], false, tch::Kind::Float);  // Convert array to slice
        
        // Map tokens back to their original words
        let token_ids: Vec<i64> = tokenized_input.token_ids.iter().map(|id| *id).collect();
        let tokens: Vec<String> = self.tokenizer.decode_list(&[token_ids], true, true);


        // Filter out special tokens ([CLS], [SEP], [PAD])
        let valid_tokens_attention: Vec<(String, f32)> = tokens
            .into_iter()
            .zip(attention_scores.into_iter())
            .filter(|(token, _)| !["[CLS]", "[SEP]", "[PAD]"].contains(&token.as_str()))
            .collect();

        // Separate tokens and their scores
        let (valid_tokens, valid_attention_scores): (Vec<_>, Vec<_>) = valid_tokens_attention.into_iter().unzip();

        Ok((valid_tokens, valid_attention_scores))
    }
}


fn download_and_save_model(save_path: &str) -> Result<()> {
    // Check if the model already exists and has contents
    let distillbert_dir = format!("{}/distillbert", save_path);

    // Create the directory if it doesn't exist
    if !Path::new(&distillbert_dir).exists() {
        create_dir_all(&distillbert_dir)?;
        println!("Created directory: {}", distillbert_dir);
    }
    println!("Downloading and saving model to {}...", save_path);

    // Downloading config, vocab, and weights
    let config_resource = RemoteResource::from_pretrained(rust_bert::distilbert::DistilBertConfigResources::DISTIL_BERT);
    let vocab_resource = RemoteResource::from_pretrained(rust_bert::distilbert::DistilBertVocabResources::DISTIL_BERT);
    let weights_resource = RemoteResource::from_pretrained(rust_bert::distilbert::DistilBertModelResources::DISTIL_BERT);

    // Save the resources locally if not already present
    let config_path = config_resource.get_local_path()?;
    let vocab_path = vocab_resource.get_local_path()?;
    let weights_path = weights_resource.get_local_path()?;

    // Check if config, vocab, or weights files exist before copying
    let config_dest = format!("{}/distillbert/config.json", save_path);
    let vocab_dest = format!("{}/distillbert/vocab.txt", save_path);
    let weights_dest = format!("{}/distillbert/model.ot", save_path);

    if !Path::new(&config_dest).exists() {
        fs::copy(config_path, &config_dest)?;
        println!("Config saved to {}", config_dest);
    } else {
        println!("Config already exists at {}. Skipping.", config_dest);
    }

    if !Path::new(&vocab_dest).exists() {
        fs::copy(vocab_path, &vocab_dest)?;
        println!("Vocab saved to {}", vocab_dest);
    } else {
        println!("Vocab already exists at {}. Skipping.", vocab_dest);
    }

    if !Path::new(&weights_dest).exists() {
        fs::copy(weights_path, &weights_dest)?;
        println!("Weights saved to {}", weights_dest);
    } else {
        println!("Weights already exist at {}. Skipping.", weights_dest);
    }

    println!("Model successfully saved to {}", save_path);
    Ok(())
}

pub async fn get_attention_scores(text: &str) -> Result<()> {
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
        let (tokens, attention_scores) = attention_calculator.calculate_attention_scores(&text_clone)?;
        info!("Tokens: {:?}", tokens);

        Ok((tokens, attention_scores))
    })
    .await??;

    // Output the tokens and attention scores
    debug!("Tokens: {:?}", tokens);
    debug!("Attention scores: {:?}", attention_scores);

    Ok(())
}