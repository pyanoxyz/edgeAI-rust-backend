use std::error::Error;
use dirs;
use std::path::{Path, PathBuf};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel};
use tch::Device;
use std::fs::{self, File};
use std::sync::{Mutex, Arc};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use std::io::Cursor;
use log::{info, error};
use dirs::home_dir;
const BASE_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/";

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
}

impl EmbeddingsManager {
    pub fn new(save_path: &str) -> Self {
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let save_path = home_dir.join(save_path);

        Self {
            save_path,
        }
    }

    fn download_file(&self, client: &Client, file_url: &str, save_path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}{}", BASE_URL, file_url);
        let file_path = save_path.join(file_url);

        if let Some(parent_dir) = file_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        if !file_path.exists() {
            info!("Downloading sentence embedding model {}...", file_url);
            let response = client.get(&url).send()?;
            if !response.status().is_success() {
                return Err(format!("Failed to download file: {}. Status: {}", file_url, response.status()).into());
            }

            let content = response.bytes()?;
            let mut dest_file = File::create(&file_path)?;
            let mut content_reader = Cursor::new(content);
            std::io::copy(&mut content_reader, &mut dest_file)?;

            info!("Embedding model downloaded: {}", file_url);
        } else {
            info!("Embedding Model {} already exists, skipping download.", file_url);
        }

        Ok(())
    }

    fn download_files(&self, save_path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        for file in FILES {
            self.download_file(&client, file, save_path)?;
        }
        Ok(())
    }

    fn ensure_model_files(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let save_path = Path::new(&self.save_path);
        let files_exist = FILES.iter().all(|file| save_path.join(file).exists());

        if !files_exist {
            info!("Embedding model files are missing, downloading...");
            if !save_path.exists() {
                fs::create_dir_all(save_path)?;
            }
            self.download_files(save_path)?;
        } else {
            info!("Embedding Model files already exist, skipping download.");
        }

        Ok(())
    }

    fn initialize_model(&self) -> Result<SentenceEmbeddingsModel, Box<dyn Error + Send + Sync>> {
        self.ensure_model_files()?;

        let model = SentenceEmbeddingsBuilder::local(self.save_path.clone())
            .with_device(Device::Cpu)
            .create_model()?;

        info!("Embedding Model loaded successfully from {}.", self.save_path.display());

        Ok(model)
    }
}

// The Lazy initialization will ensure that the model is loaded only once during the application's lifecycle.
static EMBEDDINGS_MODEL: Lazy<Result<Arc<Mutex<SentenceEmbeddingsModel>>, Box<dyn Error + Send + Sync>>> = Lazy::new(|| {
    let home_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Failed to retrieve home directory"))?;
    let models_dir = home_dir.join(".pyano/models/embed_model");

    // Ensure the model directory exists
    fs::create_dir_all(&models_dir)?;

    // Ensure the model is downloaded
    let models_dir_str = models_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert PathBuf to str"))?
        .to_string();

    let manager = EmbeddingsManager::new(&models_dir_str);
    let model = manager.initialize_model()?;
    Ok(Arc::new(Mutex::new(model)))
});

pub async fn generate_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
    let text_owned = text.to_string();
    info!("Generate embedding for Length {}", text.len());

    // Use spawn_blocking to run blocking code
    let embedding = tokio::task::spawn_blocking(move || {
        // Access the model
        let embeddings_model = EMBEDDINGS_MODEL.as_ref().map_err(|e| {
            error!("Failed to initialize embeddings model: {}", e);
            "Failed to initialize embeddings model"
        })?;

        let model_guard = embeddings_model.lock().unwrap();  // Safely access the model
        let embeddings = model_guard.encode(&[&text_owned])?;
        Ok::<Vec<f32>, Box<dyn Error + Send + Sync>>(embeddings[0].clone())
    })
    .await??;  // This is correct; the first ? unwraps the Result from spawn_blocking, and the second ? handles the Result from the closure.

    Ok(embedding)
}