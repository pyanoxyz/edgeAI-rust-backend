use std::fs::{self};
use std::error::Error;
use log::debug;
use dirs::home_dir;
use std::path::{Path, PathBuf};
use fastembed::{TextRerank, RerankInitOptions, RerankerModel, RerankResult};

pub struct RerankManager {
    save_path: PathBuf,
    model: Option<TextRerank>,
}
impl RerankManager {
    // Constructor to create a new instance of EmbeddingsManager
    pub fn new(save_path: &str) -> Self {
        let home_dir = home_dir().expect("Unable to get home directory");
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
        let init_options = RerankInitOptions::new(RerankerModel::BGERerankerBase)
            .with_cache_dir(self.save_path.clone()); // Set cache directory

        // Load model using the custom InitOptions
        let model = TextRerank::try_new(init_options)?;

        self.model = Some(model);
        debug!("Rerank Model loaded and saved to {} successfully.", save_path.display());
        Ok(())
    }

    pub fn rerank_documents(&self, query: &str, documents: Vec<&str>) -> Result<Vec<RerankResult>, Box<dyn Error>> {
        if let Some(ref model) = self.model {
            let results = model.rerank(query, documents, true, None)?;
        
            Ok(results)
        } else {
            Err(Box::from("Model is not loaded"))
        }
    }
}

// Function to create embeddings for a given text, which can be imported from other modules
pub async fn rerank_documents(query: &str, documents: Vec<&str>) -> Result<Vec<RerankResult>, Box<dyn Error>> {
    // Create the model manager instance
    let mut model_manager = RerankManager::new(".pyano/models");

    // Load the model (this will download the model if it’s not already saved)
    model_manager.load_model().await?;

    // Create text embedding for the given sentence
    let reranked_documents = model_manager.rerank_documents(query, documents)?;
    debug!("Documents has been rerabked {:?}", reranked_documents);
    Ok(reranked_documents)
}