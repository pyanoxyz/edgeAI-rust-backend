
use std::sync::{Arc, Mutex};
use super::chat_types::RequestType;
use log::{error, debug};
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
use crate::database::db_config::DB_INSTANCE;
use std::time::{Duration, Instant};
use std::future::Future;

pub async fn handle_stream_completion(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    ts_session_id: Arc<Mutex<String>>,
    ts_prompt: Arc<Mutex<String>>,
    request_type: RequestType,
) {
    if let Ok(_) = rx.await {
        let accumulated_content_final = accumulated_content.lock().unwrap().clone();

        // let summary = summarize_text(&accumulated_content_final).await.unwrap();
        let prompt = match ts_prompt.lock() {
            Ok(locked_prompt) => locked_prompt.clone(),
            Err(e) => {
                error!("Failed to acquire lock on prompt: {:?}", e);
                return;
            }
        };

        let prompt_n_response = prompt.clone() + &accumulated_content_final;
        // let result = get_attention_scores(&accumulated_content_final).await;
        let (result, duration) = measure_time_async(||  get_attention_scores(&prompt_n_response)).await;

        let tokens = match result {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Error while unwrapping tokens: {:?}", e);
                return;
            }
        };
        debug!("Time elapsed in compressing result {:?}", duration);
        // let embeddings_result = generate_text_embedding(&accumulated_content_final).await;
        let (embeddings_result, duration) = measure_time_async(|| generate_text_embedding(&prompt_n_response)).await;

        let embeddings = match embeddings_result {
            Ok(embeddings_value) => embeddings_value,
            Err(_) => {
                error!("Failed to generate embeddings");
                return;
            }
        };
        debug!("Time elapsed in generating embeddings {:?}", duration);

        let compressed_prompt_response = tokens.join(" ");

        let session_id = match ts_session_id.lock() {
            Ok(locked_session_id) => locked_session_id.clone(),
            Err(e) => {
                error!("Failed to acquire lock on session_id: {:?}", e);
                return;
            }
        };

        let db_response = DB_INSTANCE.store_chats(
            "user_id",
            &session_id,
            &prompt,
            &compressed_prompt_response,
            &accumulated_content_final,
            &embeddings,
            request_type.to_string(),
        );

        match db_response {
            Ok(_) => {
                debug!(
                    "DB Update successful for chat for session_id {}",
                    session_id
                );
            }
            Err(err) => {
                error!(
                    "Error updating chat for session_id {}: {:?}",
                    session_id, err
                );
            }
        }
    }
}

/// Measures the time taken to execute an asynchronous function.
///
/// # Arguments
///
/// * `func` - A closure or function that returns a `Future` when called.
///
/// # Returns
///
/// A `Future` that, when awaited, yields a tuple containing:
/// - The result of the asynchronous function execution.
/// - The `Duration` representing the time taken to execute the function.
pub async fn measure_time_async<T, F, Fut>(func: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let start = Instant::now();
    let result = func().await;
    let duration = start.elapsed();
    (result, duration)
}