use std::error::Error;
use log::debug;
use dirs;
use std::path::{Path, PathBuf};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel};
use tch::Device;
use tokio::task;
use std::env;
use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::copy;

const BASE_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L12-v2/resolve/main/";

const FILES: &[&str] = &[
    "1_Pooling/config.json",
    "config.json",
    "config_sentence_transformers.json",
    "data_config.json",
    "modules.json",
    "rust_model.ot",
    "sentence_bert_config.json",
    "special_tokens_map.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "vocab.txt",
];

pub struct EmbeddingsManager {
    save_path: PathBuf,
    model: Option<SentenceEmbeddingsModel>,
    initialized: bool, // Flag to check whether download has happened
}

impl EmbeddingsManager {
    // Constructor to create a new instance of EmbeddingsManager
    pub fn new(save_path: &str) -> Self {
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let save_path = home_dir.join(save_path);

        // Initialize with the assumption that the model hasn't been loaded or downloaded yet
        Self {
            save_path,
            model: None,
            initialized: false,
        }
    }

    fn download_file(&self, client: &Client, file_url: &str, save_path: &Path) -> Result<(), Box<dyn Error+Send+Sync>> {
        let url = format!("{}{}", BASE_URL, file_url);
        let file_path = save_path.join(file_url);
    
        // Ensure the parent directory exists
        if let Some(parent_dir) = file_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }
    
        // Download the file only if it doesn't exist
        if !file_path.exists() {
            println!("Downloading {}...", file_url);
            let mut response = client.get(&url).send()?;
            if !response.status().is_success() {
                return Err(format!("Failed to download file: {}", file_url).into());
            }
    
            // Write the downloaded content to the file
            let mut dest_file = File::create(&file_path)?;
            copy(&mut response, &mut dest_file)?;
            println!("Downloaded: {}", file_url);
        } else {
            println!("File {} already exists, skipping download.", file_url);
        }
    
        Ok(())
    }

    fn download_files(&self, save_path: &Path) -> Result<(), Box<dyn Error+Send+Sync>> {
        let client = Client::new();
    
        for file in FILES {
            self.download_file(&client, file, save_path)?;
        }
    
        Ok(())
    }

    // Lazy initialization to check/download model only the first time it's accessed
    fn initialize(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // If already initialized, do nothing
        if self.initialized {
            return Ok(());
        }

        let save_path = Path::new(&self.save_path);
        
        // Create the directory if it doesn't exist
        if !save_path.exists() {
            fs::create_dir_all(save_path)?;
        }

        // Download model files if not already present
        self.download_files(save_path)?;

        // Mark as initialized after download is completed
        self.initialized = true;
        Ok(())
    }

    // Function to load the model from the save path (cache) or download if not found
    pub fn load_model(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Ensure that the model is initialized (downloaded) before trying to load it
        self.initialize()?;

        // Load the model from the local path (save_path)
        let model = SentenceEmbeddingsBuilder::local(self.save_path.to_path_buf())
            .with_device(Device::Cpu)  // Change to Cuda if needed
            .create_model()?;

        self.model = Some(model);
        debug!("Model loaded successfully from {}.", self.save_path.display());
    
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
        let mut model_manager = EmbeddingsManager::new(".pyano/models/embed_model");

        // Load the model (this will download the model if it's not already saved)
        model_manager.load_model()?;

        // Create text embedding for the given sentence
        model_manager.create_text_embedding(&text_owned)
    })
    .await??;

    Ok(embedding)
}
