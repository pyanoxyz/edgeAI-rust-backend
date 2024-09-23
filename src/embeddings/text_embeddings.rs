use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::error::Error;
use log::debug;
use dirs;
use std::path::{Path, PathBuf};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};


#[derive(Serialize, Deserialize)]
struct Embedding {
    data: Vec<f32>,
}

pub struct EmbeddingsManager {
    save_path: PathBuf,
    model: Option<TextEmbedding>,
}
impl EmbeddingsManager {
    // Constructor to create a new instance of EmbeddingsManager
    pub fn new(save_path: &str) -> Self {
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let save_path = home_dir.join(save_path);
        Self {
            save_path,
            model: None,
        }
    }
    // Function to load the model to the specified directory
    pub async fn load_model(&mut self) -> Result<(), Box<dyn Error>> {
        let save_path = Path::new(&self.save_path);
        if !save_path.exists() {
            // Create the directory if it doesn't exist
            fs::create_dir_all(save_path)?;
        }


        // Setting up the InitOptions with model_name and cache_dir
        let init_options = InitOptions::new(EmbeddingModel::AllMiniLML12V2)
            .with_cache_dir(self.save_path.clone()); // Set cache directory

        // Load model using the custom InitOptions
        let model = TextEmbedding::try_new(init_options)?;

        self.model = Some(model);
        debug!("Model loaded and saved to {} successfully.", save_path.display());
        Ok(())
    }

    // Function to create text embeddings
    pub fn create_text_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        if let Some(ref model) = self.model {
            let embeddings = model.embed(vec![text], None)?;
            Ok(embeddings[0].clone())
        } else {
            Err(Box::from("Model is not loaded"))
        }
    }
}

// Function to create embeddings for a given text, which can be imported from other modules
pub async fn generate_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    // Create the model manager instance
    let mut model_manager = EmbeddingsManager::new(".pyano/models");

    // Load the model (this will download the model if it’s not already saved)
    model_manager.load_model().await;

    // Create text embedding for the given sentence
    let embedding = model_manager.create_text_embedding(text)?;
    debug!("Embeddings has been created {:?}", embedding);
    Ok(embedding)
}