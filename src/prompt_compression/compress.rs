use rust_bert::distilbert::{DistilBertConfig, DistilBertModelMaskedLM};
use rust_bert::resources::{RemoteResource, ResourceProvider, LocalResource};
use rust_bert::Config;
use rust_tokenizers::tokenizer::{BertTokenizer, Tokenizer, TruncationStrategy};
use tch::{nn, Device, Tensor, no_grad};
use std::path::Path;
use std::fs;
use anyhow::Result;
use std::env;
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

        let config_path = format!("{}/config.json", model_path);
        let vocab_path = format!("{}/vocab.txt", model_path);
        let weights_path = format!("{}/model.ot", model_path);

        // Load DistilBERT config
        let config = DistilBertConfig::from_file(config_path);

        // Load DistilBERT model
        let mut vs = nn::VarStore::new(device);
        let model = DistilBertModelMaskedLM::new(&vs.root(), &config);

        // Load tokenizer
        let tokenizer = BertTokenizer::from_file(vocab_path, true, true)?;

        // Load model weights
        vs.load(weights_path)?;

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

        let attention = outputs.all_attentions.unwrap().last().unwrap().copy();
        
        // Average attention across all heads
        let attention_weights = attention.mean_dim(&[1i64][..], false, tch::Kind::Float);  // Convert array to slice
        let attention_scores: Vec<f32> = Vec::<f32>::try_from(attention_weights.squeeze().to_kind(tch::Kind::Float).contiguous())?;
        
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
    if Path::new(save_path).exists() && fs::read_dir(save_path)?.count() > 0 {
        println!("Model for attention already exists at {}. Skipping download.", save_path);
        return Ok(());
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

    // Copy the files to the destination folder if needed
    fs::copy(config_path, format!("{}/config.json", save_path))?;
    fs::copy(vocab_path, format!("{}/vocab.txt", save_path))?;
    fs::copy(weights_path, format!("{}/model.ot", save_path))?;

    println!("Model successfully saved to {}", save_path);
    Ok(())
}

fn main() -> Result<()> {
    let home_dir = env::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
    let pyano_models_dir = home_dir.join(".pyano/models");

    // Ensure the model directory exists
    fs::create_dir_all(&pyano_models_dir)?;
    // Ensure the model is downloaded
    let pyano_models_dir_str = pyano_models_dir.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?;

    download_and_save_model(pyano_models_dir_str)?;

    // Create the AttentionCalculator
    let attention_calculator = AttentionCalculator::new(pyano_models_dir_str)?;

    // Calculate attention scores for a sample text
    let text = "Hello, how are you?";
    let (tokens, attention_scores) = attention_calculator.calculate_attention_scores(text)?;

    // Output the tokens and attention scores
    println!("Tokens: {:?}", tokens);
    println!("Attention scores: {:?}", attention_scores);

    Ok(())
}