use std::error::Error;
use log::error;
use dirs::home_dir;
use std::path::PathBuf;
use fastembed::{TextRerank, RerankInitOptions, RerankerModel, RerankResult};
use std::sync::{Mutex, Arc};
use once_cell::sync::Lazy;

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
    pub fn load_model(&mut self) -> Result<(), Box<dyn Error>> {
        // Setting up the InitOptions with model_name and cache_dir
        let init_options = RerankInitOptions::new(RerankerModel::JINARerankerV1TurboEn)
            .with_cache_dir(self.save_path.clone()); // Set cache directory

        // Load model using the custom InitOptions
        let model = TextRerank::try_new(init_options)?;

        self.model = Some(model);
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

static RERANK_MANAGER: Lazy<Result<Arc<Mutex<RerankManager>>, Box<dyn Error + Send + Sync>>> = Lazy::new(|| {
    let home_dir = home_dir().expect("Failed to retrieve home directory");
    let rerank_dir = home_dir.join(".pyano/models/reranker");

    // Ensure the model directory exists
    std::fs::create_dir_all(&rerank_dir).expect("Failed to create model directory");

    let rerank_dir_str = rerank_dir
        .to_str()
        .expect("Failed to convert PathBuf to str")
        .to_string();

    let mut model_manager: RerankManager = RerankManager::new(&rerank_dir_str);
    model_manager.load_model().map_err(|e| format!("Failed to load Reranker model: {}", e))?;
    Ok(Arc::new(Mutex::new(model_manager)))
});

pub async fn rerank_documents(
    query: &str,
    documents: Vec<String>,
) -> Result<Vec<(String, usize, f32)>, Box<dyn Error + Send + Sync>> {
    // Clone the query and documents to own their data
    let query_cloned = query.to_owned();
    let documents_cloned: Vec<String> = documents.iter().map(|s| s.to_owned()).collect();

    // Use spawn_blocking to run blocking code
    let reranked_documents = tokio::task::spawn_blocking(move || {
        // Access the model
        let reranker_manager_guarded = RERANK_MANAGER.as_ref().map_err(|e| {
            error!("Failed to initialize rerank manager {}", e);
            // Return the actual error as a boxed error instead of a string
            Box::<dyn Error + Send + Sync>::from("Failed to initialize embeddings model")
        })?;

        // Safely access the model with proper error handling
        let rerank_manager = reranker_manager_guarded.lock().map_err(|e| {
            error!("Failed to acquire lock on rerank manager: {}", e);
            Box::<dyn Error + Send + Sync>::from("Failed to acquire lock on rerank manager")
        })?;

        // Use references to the owned data
        let query_ref = &query_cloned;
        let documents_refs: Vec<&str> = documents_cloned.iter().map(|s| s.as_str()).collect();

        // Perform reranking and handle any potential error
        let reranked_documents = rerank_manager.rerank_documents(query_ref, documents_refs)
            .map_err(|e| {
                error!("Reranking failed: {}", e);
                Box::<dyn Error + Send + Sync>::from("Reranking failed")
            })?;

        let result: Vec<(String, usize, f32)> = reranked_documents
            .iter()
            .map(|rerank| {
                (
                    rerank.document.as_ref().unwrap().to_owned(),
                    rerank.index,
                    rerank.score,
                )
            })
            .collect();

        // Return the result inside the closure
        Ok::<Vec<(String, usize, f32)>, Box<dyn Error + Send + Sync>>(result)
    })
    .await??; // Unwrap the results from spawn_blocking and the Result

    Ok(reranked_documents)
}