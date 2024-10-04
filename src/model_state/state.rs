use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use std::sync::{ Arc, Mutex };
use tokio::task::JoinHandle;


#[derive(Clone)] // Add this line
pub struct ModelState {
    pub model_process: Arc<TokioMutex<Option<JoinHandle<()>>>>, // Ensure Arc is used for both fields
    pub model_pid: Arc<Mutex<Option<u32>>>,
}