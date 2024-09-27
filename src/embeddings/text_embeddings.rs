use std::error::Error;
use dirs;
use std::path::{Path, PathBuf};
use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel};
use tch::Device;
use tokio::task;
use std::fs::{self, File};
use std::sync::{Once, Mutex, Arc};
use reqwest::blocking::Client;
use std::io::Cursor;
use tokio::task::LocalSet;
use log::info;
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
            println!("Downloading {}...", file_url);
            let response = client.get(&url).send()?;
            if !response.status().is_success() {
                return Err(format!("Failed to download file: {}. Status: {}", file_url, response.status()).into());
            }

            let content = response.bytes()?;
            let mut dest_file = File::create(&file_path)?;
            let mut content_reader = Cursor::new(content);
            std::io::copy(&mut content_reader, &mut dest_file)?;

            println!("Downloaded: {}", file_url);
        } else {
            println!("File {} already exists, skipping download.", file_url);
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
            println!("Some model files are missing, downloading...");
            if !save_path.exists() {
                fs::create_dir_all(save_path)?;
            }
            self.download_files(save_path)?;
        } else {
            println!("Model files already exist, skipping download.");
        }

        Ok(())
    }

    fn initialize_model(&self) -> Result<SentenceEmbeddingsModel, Box<dyn Error + Send + Sync>> {
        self.ensure_model_files()?;

        let model = SentenceEmbeddingsBuilder::local(self.save_path.clone())
            .with_device(Device::Cpu)
            .create_model()?;

        println!("Model loaded successfully from {}.", self.save_path.display());

        Ok(model)
    }
}

// Global synchronization objects to initialize the model only once
static INIT: Once = Once::new();
static mut EMBEDDINGS_MODEL: Option<Arc<Mutex<SentenceEmbeddingsModel>>> = None;

// Function to ensure model is initialized only once
fn ensure_initialized(manager: &EmbeddingsManager) -> Result<Arc<Mutex<SentenceEmbeddingsModel>>, Box<dyn Error + Send + Sync>> {
    unsafe {
        INIT.call_once(|| {
            if let Ok(model) = manager.initialize_model() {
                EMBEDDINGS_MODEL = Some(Arc::new(Mutex::new(model)));
            }
        });

        // Return the initialized model or error if it failed
        EMBEDDINGS_MODEL.clone().ok_or_else(|| "Failed to initialize the model.".into())
    }
}

// This function uses LocalSet to ensure single-threaded execution
pub async fn generate_text_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
    let text_owned = text.to_string();
    info!("Generate embedding for Length {}", text.len());
    // Use LocalSet to run tasks on a single thread
    let local = LocalSet::new();
    let embedding = local
        .run_until(task::spawn_blocking(move || {
            let manager = EmbeddingsManager::new(".pyano/models/embed_model");

            // Ensure the model is initialized only once
            let model = ensure_initialized(&manager)?;

            let model_guard = model.lock().unwrap();  // Safely access the model
            let embeddings = model_guard.encode(&[&text_owned])?;
            Ok::<Vec<f32>, Box<dyn Error + Send + Sync>>(embeddings[0].clone())
        }))
        .await??;

    Ok(embedding)
}
