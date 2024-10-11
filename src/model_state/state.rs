use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use std::sync::{ Arc, Mutex };
use tokio::task::JoinHandle;
use serde::{Deserialize, Serialize};

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

#[derive(Clone)] // Add this line
pub struct ModelState {
    pub model_process: Arc<TokioMutex<Option<JoinHandle<()>>>>, // Ensure Arc is used for both fields
    pub model_pid: Arc<Mutex<Option<u32>>>,
}
