use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use std::sync::{ Arc, Mutex };
use tokio::task::JoinHandle;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigSection {
    pub model_name: String,
    pub model_url: String,
    pub model_size: f64,
    pub system_prompt: String,
    pub ctx_size: u32,
    pub gpu_layers_offloading: i32,
    pub batch_size: u32,
    pub mlock: bool,
    pub mmap: bool,
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error(
        "Model {0} referenced in coderModels does not exist in models map"
    )] InvalidModelReference(String),
    #[error("Deserialization error: {0}")] DeserializationError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub ctx_size: i32,
    pub gpu_layers_offloading: i32,
    pub batch_size: i32,
    pub mlock: bool,
    pub mmap: bool,
    pub system_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    pub required: bool,
    #[serde(rename = "modelName")]
    pub model_name: String,
    pub category: String,
    #[serde(rename = "modelConfig")]
    pub model_config: Option<ModelConfig>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfigJson {
    #[serde(rename = "coderModels")]
    pub coder_models: Vec<String>,
    pub models: HashMap<String, Model>,
}

impl AppConfigJson {
    pub fn from_str(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(json)?)
    }

    // Helper method to get a specific model
    pub fn get_model(&self, name: &str) -> Option<&Model> {
        self.models.get(name)
    }

    // Helper method to check if a model exists
    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    
}

#[derive(Clone)] // Add this line
pub struct ModelState {
    pub model_process: Arc<TokioMutex<Option<JoinHandle<()>>>>, // Ensure Arc is used for both fields
    pub model_pid: Arc<Mutex<Option<u32>>>,
    pub model_type: Arc<Mutex<String>>,
}
