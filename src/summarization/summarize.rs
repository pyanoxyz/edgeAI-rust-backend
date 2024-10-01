use anyhow::Result;
use rust_bert::pipelines::summarization::{SummarizationConfig, SummarizationModel};
use rust_bert::resources::{LocalResource, RemoteResource, ResourceProvider}; // Import ResourceProvider for get_local_path
use rust_bert::Config;
use rust_bert::bart::BartConfig;
use rust_bert::pipelines::common::ModelResource;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};
use log::debug;
use rust_bert::bart::{BartMergesResources, BartConfigResources, BartVocabResources, BartModelResources};
pub struct SummarizationCalculator {
    model: SummarizationModel,
}

impl SummarizationCalculator {
    pub fn new(model_path: &str) -> Result<Self> {
        // Specify the local resources for config, merges, vocab, and weights
        let config_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/config.json", model_path)),
        };
        let vocab_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/vocab.txt", model_path)),
        };
        let merges_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/merges.txt", model_path)),
        };
        let weights_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/pytorch_model.bin", model_path)),
        };

        // Create a summarization configuration
        let summarization_config = SummarizationConfig {
            model_resource: ModelResource::Torch(Box::new(weights_resource)),
            config_resource: Box::new(config_resource),
            vocab_resource: Box::new(vocab_resource),
            merges_resource: Some(Box::new(merges_resource)),
            ..Default::default()
        };

        // Initialize the SummarizationModel with the custom config
        let model = SummarizationModel::new(summarization_config)?;

        // Return the struct with the loaded model
        Ok(SummarizationCalculator { model })
    }

    pub fn summarize(&self, input_text: &str) -> Result<String> {
        // Summarize the input text and handle the result
        let summaries_result = self.model.summarize(&[input_text]);
        
        // Unwrap the Result to access the Vec<String>, or return the error if summarization fails
        let summaries = summaries_result?;
        
        // Return the first summary (assuming single input)
        Ok(summaries[0].clone())
    }
}


fn download_and_save_model(save_path: &str) -> Result<()> {
    // Define paths for config, vocab, and weights
    let config_dest = format!("{}/config.json", save_path);
    let vocab_dest = format!("{}/vocab.json", save_path);
    let merges_dest = format!("{}/merges.txt", save_path);
    let weights_dest = format!("{}/pytorch_model.bin", save_path);

    // Check if any of the model files (config, vocab, weights) already exist
    if Path::new(&config_dest).exists() && Path::new(&vocab_dest).exists() && Path::new(&weights_dest).exists() {
        debug!("BART model files already exist at {}. Skipping download.", save_path);
        return Ok(());
    }

    // Create the directory if it doesn't exist
    let bart_dir = format!("{}/bart-large-cnn", save_path);
    if !Path::new(&bart_dir).exists() {
        create_dir_all(&bart_dir)?;
        debug!("Created directory: {}", bart_dir);
    }

    // Downloading config, vocab, and weights only if they don't exist
    if !Path::new(&config_dest).exists() {
        let config_resource = RemoteResource::from_pretrained(BartConfigResources::BART);
        let config_path = config_resource.get_local_path()?;
        fs::copy(config_path, &config_dest)?;
        debug!("Config saved to {}", config_dest);
    } else {
        debug!("Config already exists at {}. Skipping.", config_dest);
    }

    if !Path::new(&vocab_dest).exists() {
        let vocab_resource = RemoteResource::from_pretrained(BartVocabResources::BART);
        let vocab_path = vocab_resource.get_local_path()?;
        fs::copy(vocab_path, &vocab_dest)?;
        debug!("Vocab saved to {}", vocab_dest);
    } else {
        debug!("Vocab already exists at {}. Skipping.", vocab_dest);
    }

    if !Path::new(&merges_dest).exists() {
        let merges_resource = RemoteResource::from_pretrained(BartMergesResources::BART);
        let merges_path = merges_resource.get_local_path()?;
        fs::copy(merges_path, &merges_dest)?;
        debug!("Merges saved to {}", merges_dest);
    } else {
        debug!("Merges already exists at {}. Skipping.", merges_dest);
    }

    if !Path::new(&weights_dest).exists() {
        let weights_resource = RemoteResource::from_pretrained(BartModelResources::BART);
        let weights_path = weights_resource.get_local_path()?;
        fs::copy(weights_path, &weights_dest)?;
        debug!("Weights saved to {}", weights_dest);
    } else {
        debug!("Weights already exist at {}. Skipping.", weights_dest);
    }

    debug!("Model successfully saved to {}", save_path);
    Ok(())
}


fn main() -> Result<()> {
    // Specify a path where the BART model is stored locally
    let model_path = "~/.pyano/models/summarization";
    
    // Initialize the summarizer with the custom model path
    let summarizer = SummarizationCalculator::new(model_path)?;

    let input_text = "OpenAI is an AI research lab that focuses on developing friendly AI. It was founded in 2015 by Elon Musk, Sam Altman, and others.";
    let summary = summarizer.summarize(input_text)?;

    println!("Summary: {}", summary);

    Ok(())
}
