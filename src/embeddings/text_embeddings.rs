use std::fs;
use std::error::Error;
use log::debug;
use dirs;
use std::path::{Path, PathBuf};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType};
use tch::Device;
use tokio::task;
use std::env;

pub struct EmbeddingsManager {
    save_path: PathBuf,
    model: Option<SentenceEmbeddingsModel>,
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

    // Function to load the model from the save path (cache) or download if not found
    pub fn load_model(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let save_path = Path::new(&self.save_path);
        
        // Create the directory if it doesn't exist
        if !save_path.exists() {
            fs::create_dir_all(save_path)?;
        }
    
        // Set the `HF_HOME` environment variable to point to save_path for model loading
        std::env::set_var("HF_HOME", self.save_path.to_str().unwrap());
    
        // Check if the model already exists in save_path
        let model_files_exist = save_path.join("config.json").exists() && save_path.join("pytorch_model.bin").exists();
    
        // Create the model from local path if available, otherwise download it
        let model = if model_files_exist {
            // Load the model from the local path (save_path)
            SentenceEmbeddingsBuilder::local(save_path.to_path_buf())
                .with_device(Device::Cpu)  // Change to Cuda if needed
                .create_model()?
        } else {
            // Download and cache the model at save_path
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
                .with_device(Device::Cpu)  // Change to Cuda if needed
                .create_model()?
        };
    
        self.model = Some(model);
        debug!("Model loaded successfully from {}.", save_path.display());
    
        Ok(())
    }

    // Function to create text embeddings
    pub fn create_text_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
        if let Some(ref model) = self.model {
            let embeddings = model.encode(&[text])?;  // Encode the text using the Sentence Embeddings model
            Ok(embeddings[0].clone())
        } else {
            Err(Box::from("Model is not loaded"))
        }
    }
}

// Function to create embeddings for a given text, which can be imported from other modules
pub async fn generate_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error +Send +Sync>> {
    // Spawn the blocking task to avoid blocking the async runtime
    let text_owned = text.to_string();
    let home_dir = env::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
    let pyano_models_dir = home_dir.join(".pyano/models");
   // Convert the Pyano models directory path to a string and ensure it's valid
   let pyano_models_dir_str: String = pyano_models_dir
   .to_str()
   .ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?
   .to_string(); // Convert to owned String

    let embedding = task::spawn_blocking(move || {
        let mut model_manager = EmbeddingsManager::new(&pyano_models_dir_str);

        // Load the model (this will download the model if it's not already saved)
        model_manager.load_model()?;

        // Create text embedding for the given sentence
        model_manager.create_text_embedding(&text_owned)
    })
    .await??;

    Ok(embedding)
}