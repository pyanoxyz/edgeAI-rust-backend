use rust_bert::pipelines::sentence_embeddings::SentenceEmbeddingsModel;
use rust_bert::resources::{RemoteResource, Resource};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::error::Error;
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType};
use log::debug;
use dirs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Embedding {
    data: Vec<f32>,
}

pub struct EmbeddingsManager {
    model_name: Option<String>,
    save_path: PathBuf,
    model: Option<SentenceEmbeddingsModel>,
}

impl EmbeddingsManager {
    // Constructor to create a new instance of EmbeddingsManager
    pub fn new(model_name: Option<&str>, save_path: &str) -> Self {
        let home_dir = dirs::home_dir().expect("Unable to get home directory"); // Get the home directory
        let save_path = home_dir.join(save_path); // Append ".pyano/models" to the home directory
        Self {
            model_name: model_name.map(|name| name.to_string()), // Optional model name
            save_path,
            model: None,
        }
    }

    // Function to load the model
    // Function to load and save the model if needed
    pub fn load_model(&mut self) -> Result<(), Box<dyn Error>> {
        let save_path = Path::new(&self.save_path);
        if save_path.exists() && save_path.is_dir() && fs::read_dir(save_path)?.count() > 0 {
            debug!("Model already exists at {}. Skipping download.", save_path.display());
        } else {
            // Create the directory if it doesn't exist
            fs::create_dir_all(save_path)?;
        }

        // Use either the provided model name or a default one
        let model_name = self.model_name.clone().unwrap_or_else(|| {
            "sentence-transformers/all-MiniLM-L12-v2".to_string()
        });

        // Load the Sentence Embeddings Model from either the local directory or remote (if not available locally)
        let model = if self.model_name.is_some() {
            SentenceEmbeddingsBuilder::local(&self.save_path)
                .create_model()?
        } else {
            SentenceEmbeddingsBuilder::remote(
                SentenceEmbeddingsModelType::AllMiniLmL12V2
            ).create_model()?
        };

        self.model = Some(model);
        println!("Model loaded successfully.");
        Ok(())
    }

    // Function to create text embeddings
    pub fn create_text_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        // Ensure the model is loaded
        if let Some(ref model) = self.model {
            // Encode text into embeddings
            let embeddings = model.encode(&[text])?;
            Ok(embeddings[0].clone())
        } else {
            Err(Box::from("Model is not loaded"))
        }
    }

    // Function to simulate token creation (placeholder)
    pub fn create_tokens(&self, text: &str) -> Vec<i64> {
        // Note: Tokenization is handled internally when creating embeddings in rust-bert.
        println!("Tokenization in rust-bert is handled internally during embedding creation.");
        vec![] // Return an empty vector as a placeholder
    }
}

// Function to create embeddings for a given text, which can be imported from other modules
pub fn generate_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    // Create the model manager instance
    let mut model_manager = EmbeddingsManager::new(None, ".pyano/models");

    // Load the model (this will download the model if it’s not already saved)
    model_manager.load_model()?;

    // Create text embedding for the given sentence
    let embedding = model_manager.create_text_embedding(text)?;
    
    Ok(embedding)
}