use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use std::sync::{ Arc, Mutex };
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct InfillModelState {
    pub infill_model_process: Arc<TokioMutex<Option<JoinHandle<()>>>>, // Ensure Arc is used for both fields
    pub infill_model_pid: Arc<Mutex<Option<u32>>>,
}
