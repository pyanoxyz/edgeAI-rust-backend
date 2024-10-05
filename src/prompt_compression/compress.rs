// use rust_bert::distilbert::{DistilBertConfig, DistilBertModelMaskedLM};
use rust_bert::bert::{BertConfig, BertForMaskedLM};
use std::error::Error;
use log::error;
use rust_bert::resources::{RemoteResource, ResourceProvider};
use rust_bert::Config;
use rust_tokenizers::tokenizer::{BertTokenizer, Tokenizer, TruncationStrategy};
use tch::{nn, Device, Tensor, no_grad};
use std::path::Path;
use std::fs;
use anyhow::Result;
use log::debug;
use std::fs::create_dir_all;
use tch::IndexOp;
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
/// Directory to save the model
use std::io::{Read, Write};  // Import the required traits
use dirs::home_dir;
use std::sync::{Mutex, Arc};
use once_cell::sync::Lazy;
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
        let model = BertForMaskedLM::new(vs.root(), &config);

        // Load tokenizer
        let tokenizer = BertTokenizer::from_file(vocab_path.clone(), true, true)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer from {}: {:?}", vocab_path, e))?;
    
        // Load model weights
        vs.load(weights_path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to load model weights from {}: {:?}", weights_path, e))?;
    
        Ok(AttentionCalculator { model, tokenizer, device })
    }
    pub fn calculate_attention_scores(&self, prompt: &str, threshold: f32) -> Result<Vec<String>, anyhow::Error> {
        let mut all_tokens: Vec<String> = Vec::new();
        let mut current_start = 0;
        let chunk_size = 512;
    
        while current_start < prompt.len() {
            // Ensure we don't go out of bounds
            let end = if current_start + chunk_size > prompt.len() {
                prompt.len()
            } else {
                current_start + chunk_size
            };
    
            // Tokenize the chunk of the prompt starting from the current position
            let result: Result<(Vec<String>, Vec<f32>), anyhow::Error> = self.calculate_attention_per_chunk(&prompt[current_start..end], threshold);
    
            let (tokens, _) = match result {
                Ok((tokens, attention_scores)) => (tokens, attention_scores),
                Err(e) => {
                    println!("Error while unwrapping tokens: {:?}", e);
                    return Err(e);
                }
            };
    
            // Add the chunk tokens to the vector
            all_tokens.extend(tokens);
    
            // Move the start index forward by the length of the chunk (i.e., 512 characters)
            current_start += chunk_size;
        }
    
        Ok(all_tokens)
    }


    pub fn calculate_attention_per_chunk(&self, prompt: &str, threshold: f32) -> Result<(Vec<String>, Vec<f32>)> {
        // Tokenize input text into chunks
        // The models expects maximum 512 tokens, if you dont trucate it, you will get werid errors or the program will just panick without any errors

        let tokenized_input: rust_tokenizers::TokenizedInput = self.tokenizer.encode(prompt, None, 512,  &TruncationStrategy::LongestFirst, 0);

          // Extract token_ids from tokenized_input
        let token_ids = &tokenized_input.token_ids;
        let input_ids = Tensor::from_slice(&tokenized_input.token_ids).unsqueeze(0).to(self.device);
        let outputs = no_grad(|| {
            self.model
                .forward_t(Some(&input_ids), None, None, None, None, None, None, false)
        });
                
        let mut self_attention_scores: Vec<f32> ;
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
                .zip(self_attention_scores)
                .filter(|(token, score)| {
                    !["[CLS]", "[SEP]", "[PAD]"].contains(&token.as_str()) 
                    && !token.starts_with("##")
                    && *score > threshold
                })
                .collect();
                // Log the tokens and their attention
                let (valid_tokens, valid_attention_scores): (Vec<_>, Vec<_>) = valid_tokens_attention.into_iter().unzip();
                Ok((valid_tokens, valid_attention_scores))

            } else {
                Err(anyhow::anyhow!("No last attention scores found"))
            }
        } else {
            Err(anyhow::anyhow!("No attention output available from the model"))
        }
  
    }
}

fn modify_config_file(file_path: &str, new_key: &str, new_value: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the file exists
    if !Path::new(file_path).exists() {
        return Err("Config file not found".into());
    }

    // Open the file
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the file contents into a JSON object
    let mut config: Value = serde_json::from_str(&contents)?;

    // Check if the key already exists
    if !config.get(new_key).is_some() {
        // If the key doesn't exist, add it
        config[new_key] = json!(new_value);

        // Write the modified JSON back to the file
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;

        let updated_contents = serde_json::to_string_pretty(&config)?;
        file.write_all(updated_contents.as_bytes())?;
        println!("Key '{}' added to the config file.", new_key);
    } else {
        println!("Key '{}' already exists. No changes made.", new_key);
    }

    Ok(())
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
    let bert_dir = format!("{}/bert-bert-uncased", save_path);
    if !Path::new(&bert_dir).exists() {
        create_dir_all(&bert_dir)?;
        debug!("Created directory: {}", bert_dir);
    }

    // Downloading config, vocab, and weights only if they don't exist
    if !Path::new(&config_dest).exists() {
        let config_resource = RemoteResource::from_pretrained(rust_bert::bert::BertConfigResources::BERT);
        let config_path = config_resource.get_local_path()?;
        fs::copy(config_path, &config_dest)?;
        debug!("Config saved to {}", config_dest);
        //Ifthis key is not added to the config.json files, The model will stop giving out attention scores
        modify_config_file(&config_dest, "output_attentions", true);

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

static ATTENTION_MODEL: Lazy<Result<Arc<Mutex<AttentionCalculator>>, Box<dyn Error + Send + Sync>>> = Lazy::new(|| {
    let home_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
    let model_dir = home_dir.join(".pyano/models");

    // Ensure the model directory exists
    fs::create_dir_all(&model_dir)?;

    // Ensure the model is downloaded
    let model_dir_str = model_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?
        .to_string();
    download_and_save_model(&model_dir_str).map_err(|e| anyhow::anyhow!(e))?;

    let attention_calculator = AttentionCalculator::new(&model_dir_str).unwrap();
    println!("attention_calculator loaded successfully.");
    Ok(Arc::new(Mutex::new(attention_calculator)))
});

// pub async fn get_attention_scores(text: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
//     let home_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
//     let pyano_models_dir = home_dir.join(".pyano/models");

//     // Ensure the model directory exists
//     fs::create_dir_all(&pyano_models_dir)?;

//     // Ensure the model is downloaded
//     let pyano_models_dir_str = pyano_models_dir
//         .to_str()
//         .ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?
//         .to_string();

//     // Clone variables for the closure
//     let text_clone = text.to_string();
//     let pyano_models_dir_str_clone = pyano_models_dir_str.clone();

//     // Run the download_and_save_model in a blocking task
//     let tokens: Vec<String> = task::spawn_blocking(move ||{
//         // Download and save the model, handle any errors
//         download_and_save_model(&pyano_models_dir_str_clone).map_err(|e| anyhow::anyhow!(e))?;

//         // Create the AttentionCalculator
//         let attention_calculator = AttentionCalculator::new(&pyano_models_dir_str_clone)?;

//         // Calculate attention scores for the input text Larger value will give less content
//         let tokens = attention_calculator.calculate_attention_scores(&text_clone, 0.04)?;

//         Ok::<Vec<String>, Box<dyn Error + Send + Sync>>(tokens.clone())
//     })
//     .await??;

//     Ok(tokens)
// }


// pub async fn get_attention_scores(text: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
//     let text_owned = text.to_string();

//     // Use block_in_place to run blocking code
//     let tokens: Vec<String> = tokio::task::block_in_place(move || {
//         // Access the model
//         let model = ATTENTION_MODEL.as_ref().map_err(|e| {
//             error!("Failed to initialize attention model: {}", e);
//             "Failed to initialize attention model"
//         })?;

//         let attention_calculator = model.lock().unwrap();  // Safely access the model
//         let tokens = attention_calculator.calculate_attention_scores(&text_owned, 0.04)?;
//         Ok::<Vec<f32>, Box<dyn Error + Send + Sync>>(tokens.clone())
//     })?;

//     Ok(tokens)
// }
// pub async fn get_attention_scores(text: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
//     let text_owned = text.to_string();

//     // Use block_in_place to run blocking code
//     let tokens = tokio::task::block_in_place(move || {
//         // Access the model
//         let model = ATTENTION_MODEL.as_ref().map_err(|e| {
//             error!("Failed to initialize attention model: {}", e);
//             "Failed to initialize attention model"
//         })?;

//         let attention_calculator = model.lock().unwrap();  // Safely access the model
//         let tokens = attention_calculator.calculate_attention_scores(&text_owned, 0.04)?;
//         Ok::<Vec<String>, Box<dyn Error + Send + Sync>>(tokens)
//     })?;

//     Ok(tokens)
// }

pub async fn get_attention_scores(text: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let text_owned = text.to_string();

    // Use `spawn_blocking` to run the blocking code
    let tokens = tokio::task::spawn_blocking(move || {
        // Access the model
        let model = ATTENTION_MODEL.as_ref().map_err(|e| {
            error!("Failed to initialize attention model: {}", e);
            "Failed to initialize attention model"
        })?;

        let attention_calculator = model.lock().unwrap();  // Safely access the model
        let tokens = attention_calculator.calculate_attention_scores(&text_owned, 0.04)?;
        Ok::<Vec<String>, Box<dyn Error + Send + Sync>>(tokens)
    })
    .await??;

    Ok(tokens)
}