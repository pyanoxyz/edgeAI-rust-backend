use anyhow::Result;
use rust_bert::pipelines::summarization::{SummarizationConfig, SummarizationModel};
use log::{info, error};
use std::sync::{Mutex, Arc};
use once_cell::sync::Lazy;
use dirs::home_dir;
use std::error::Error;

pub struct SummarizationCalculator {
    model: SummarizationModel,
}

impl SummarizationCalculator {
    pub fn new() -> Result<Self> {
        // Initialize the SummarizationModel with default configuration
        let model = SummarizationModel::new(SummarizationConfig::default())?;
        info!("Model has been loaded successfully");

        Ok(SummarizationCalculator { model })
    }

    pub fn summarize(&self, input_text: &str) -> Result<String> {
        // Summarize the input text
        let summaries = self.model.summarize(&[input_text])?;

        // Return the first summary
        Ok(summaries[0].clone())
    }
}

static SUMMARIZER_MODEL: Lazy<Result<Arc<Mutex<SummarizationCalculator>>, Box<dyn Error + Send + Sync>>> = Lazy::new(|| {
    let home_dir = home_dir().expect("Failed to retrieve home directory");
    let summarization_dir = home_dir.join(".pyano/models");

    // Ensure the model directory exists
    std::fs::create_dir_all(&summarization_dir).expect("Failed to create model directory");

    let summarization_dir_str = summarization_dir
        .to_str()
        .expect("Failed to convert PathBuf to str")
        .to_string();

    // Set the RUSTBERT_CACHE environment variable to the desired directory
    std::env::set_var("RUSTBERT_CACHE", &summarization_dir_str);

    let summarization_calculator = SummarizationCalculator::new()?;
    info!("Summarization Model loaded successfully.");
    Ok(Arc::new(Mutex::new(summarization_calculator)))
});

pub async fn summarize_text(text: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let text_owned = text.to_string();

    // Use `spawn_blocking` to run the blocking code
    let summary: String = tokio::task::spawn_blocking(move || {
        // Access the model
        let model = SUMMARIZER_MODEL.as_ref().map_err(|e| {
            error!("Failed to initialize summarization model: {}", e);
            "Failed to initialize summarization model"
        })?;

        let model = model.lock().unwrap();
        let summary = model.summarize(&text_owned)?;

        Ok::<String, Box<dyn Error + Send + Sync>>(summary)
    })
    .await??;

    Ok(summary)
}