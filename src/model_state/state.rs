use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use std::sync::{ Arc, Mutex };
use tokio::task::JoinHandle;
use serde::Deserialize;




#[derive(Debug, Deserialize)]
pub struct ConfigSection {
    pub model_name: String,
    pub model_url: String,
    pub model_size: f64,
    pub ctx_size: u32,
    pub gpu_layers_offloading: i32,
    pub batch_size: u32,
    pub keep_model_in_memory: bool,
    pub mmap: bool,
}

#[derive(Clone)] // Add this line
pub struct ModelState {
    pub model_process: Arc<TokioMutex<Option<JoinHandle<()>>>>, // Ensure Arc is used for both fields
    pub model_pid: Arc<Mutex<Option<u32>>>,
}
