
use std::sync::{Arc, Mutex};
use super::chat_types::RequestType;
use log::{error, debug};
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
use crate::database::db_config::DB_INSTANCE;

pub async fn handle_stream_completion(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    ts_session_id: Arc<Mutex<String>>,
    ts_prompt: Arc<Mutex<String>>,
    request_type: RequestType,
) {
    if let Ok(_) = rx.await {
        let accumulated_content_final = accumulated_content.lock().unwrap().clone();

        let result = get_attention_scores(&accumulated_content_final).await;
        let tokens = match result {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Error while unwrapping tokens: {:?}", e);
                return;
            }
        };

        let embeddings_result = generate_text_embedding(&accumulated_content_final).await;
        let embeddings = match embeddings_result {
            Ok(embeddings_value) => embeddings_value,
            Err(_) => {
                error!("Failed to generate embeddings");
                return;
            }
        };

        let compressed_prompt = tokens.join(" ");
        debug!("Compressed Prompt {:?}", compressed_prompt);
        println!("Final accumulated content: {}", accumulated_content_final);

        let session_id = match ts_session_id.lock() {
            Ok(locked_session_id) => locked_session_id.clone(),
            Err(e) => {
                error!("Failed to acquire lock on session_id: {:?}", e);
                return;
            }
        };

        let prompt = match ts_prompt.lock() {
            Ok(locked_prompt) => locked_prompt.clone(),
            Err(e) => {
                error!("Failed to acquire lock on prompt: {:?}", e);
                return;
            }
        };

        let db_response = DB_INSTANCE.store_chats(
            "user_id",
            &session_id,
            &prompt,
            &compressed_prompt,
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
