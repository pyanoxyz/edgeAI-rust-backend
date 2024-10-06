use anyhow::Result;
use rust_bert::pipelines::summarization::{SummarizationConfig, SummarizationModel};
use rust_bert::resources::{LocalResource, RemoteResource, ResourceProvider}; // Import ResourceProvider for get_local_path

use rust_bert::pipelines::common::ModelResource;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};
use log::debug;
use std::sync::Once;
use dirs::home_dir;
use tokio::task::LocalSet;
use tokio::task;

static INIT: Once = Once::new();

pub struct SummarizationCalculator {
    model: SummarizationModel,
}

impl SummarizationCalculator {
    pub fn new(model_path: &str) -> Self {

        // Specify the local resources for config, merges, vocab, and weights
        let config_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/config.json", model_path)),
        };
        let vocab_resource = LocalResource {
            local_path: PathBuf::from(format!("{}/vocab.json", model_path)),
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
        let model = SummarizationModel::new(summarization_config).unwrap();

        // Return the struct with the loaded model
        SummarizationCalculator { model }
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



pub fn download_and_save_model(save_path: &str) -> Result<()> {
    // Use std::sync::Once to ensure this block is run only once
    INIT.call_once(|| {
        // Define paths for config, vocab, merges, and weights
        let config_dest = format!("{}/config.json", save_path);
        let vocab_dest = format!("{}/vocab.json", save_path);
        let merges_dest = format!("{}/merges.txt", save_path);
        let weights_dest = format!("{}/pytorch_model.bin", save_path);

        // Check if any of the model files (config, vocab, weights) already exist
        if Path::new(&config_dest).exists() && Path::new(&vocab_dest).exists() && Path::new(&weights_dest).exists() {
            debug!("BART model files already exist at {}. Skipping download.", save_path);
            return;
        }


        // Downloading config, vocab, and weights only if they don't exist
        if !Path::new(&config_dest).exists() {
            // let config_resource = RemoteResource::from_pretrained(rust_bert::bart::BartConfigResources::BART);
            let config_resource = RemoteResource::from_pretrained(rust_bert::bart::BartConfigResources::BART);
            if let Ok(config_path) = config_resource.get_local_path() {
                if fs::copy(config_path, &config_dest).is_ok() {
                    debug!("Config saved to {}", config_dest);
                }
            }
        } else {
            debug!("Config already exists at {}. Skipping.", config_dest);
        }

        if !Path::new(&vocab_dest).exists() {
            let vocab_resource = RemoteResource::from_pretrained(rust_bert::bart::BartVocabResources::BART);
            if let Ok(vocab_path) = vocab_resource.get_local_path() {
                if fs::copy(vocab_path, &vocab_dest).is_ok() {
                    debug!("Vocab saved to {}", vocab_dest);
                }
            }
        } else {
            debug!("Vocab already exists at {}. Skipping.", vocab_dest);
        }

        if !Path::new(&merges_dest).exists() {
            let merges_resource = RemoteResource::from_pretrained(rust_bert::bart::BartMergesResources::BART);
            if let Ok(merges_path) = merges_resource.get_local_path() {
                if fs::copy(merges_path, &merges_dest).is_ok() {
                    debug!("Merges saved to {}", merges_dest);
                }
            }
        } else {
            debug!("Merges already exist at {}. Skipping.", merges_dest);
        }

        if !Path::new(&weights_dest).exists() {
            let weights_resource = RemoteResource::from_pretrained(rust_bert::bart::BartModelResources::BART);
            if let Ok(weights_path) = weights_resource.get_local_path() {
                if fs::copy(weights_path, &weights_dest).is_ok() {
                    debug!("Weights saved to {}", weights_dest);
                }
            }
        } else {
            debug!("Weights already exist at {}. Skipping.", weights_dest);
        }

        debug!("Model successfully saved to {}", save_path);
    });

    Ok(())
}


pub async fn summarize_text(text: &str) -> Result<String> {
    let home_dir = home_dir().expect("Failed to retrieve home directory");
    let summarization_dir = home_dir.join(".pyano/models/summarization_model");    // Spawn a new thread for downloading the model and initialization
    // Ensure the model directory exists
    create_dir_all(&summarization_dir).expect("Failed to create model directory");

    // Ensure the model is downloaded
    let summarization_dir_str = summarization_dir
        .to_str()
        .expect("Failed to convert PathBuf to str")
        .to_string();
    let text_clone = text.to_owned();
    debug!("Text for sumamrization is {}", text_clone);


   // Run the summarization in a separate blocking task
   let local = LocalSet::new();
   let summary = local
   .run_until(task::spawn_blocking(move || {
       // Initialize summarizer and summarize text
       let summarizer = SummarizationCalculator::new(&summarization_dir_str);
       let summary = summarizer.summarize(&text_clone)?;
       debug!("Summary is {}", summary);
       Ok::<String, anyhow::Error>(summary) // Properly return a Result here
   }))
   .await??; 

   Ok(summary)
}