use std::fs;
use std::error::Error;
use log::debug;
use dirs;
use std::path::{Path, PathBuf};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType};
use tch::Device;
use tokio::task;

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
        if !save_path.exists() {
            // Create the directory if it doesn't exist
            fs::create_dir_all(save_path)?;
        }

        // Set the `HF_HOME` environment variable to customize the cache directory
        std::env::set_var("HF_HOME", self.save_path.to_str().unwrap());

        // Create the model
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
            .with_device(Device::Cpu) // You can change this to Cuda if you have a GPU
            .create_model()?;

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

    let embedding = task::spawn_blocking(move || {
        let mut model_manager = EmbeddingsManager::new(".pyano/models");

        // Load the model (this will download the model if it's not already saved)
        model_manager.load_model()?;

        // Create text embedding for the given sentence
        model_manager.create_text_embedding(&text_owned)
    })
    .await??;

    Ok(embedding)
}