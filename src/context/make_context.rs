

use std::error::Error;
use crate::database::db_config::DB_INSTANCE;
use std::time::{Duration, Instant};
use std::future::Future;
use crate::embeddings::text_embeddings::generate_text_embedding;
use log::{error, debug};
use crate::rerank::rerank::rerank_documents;
use std::collections::HashSet;


/// Retrieves the last `n` chats for a given session.
///
/// # Arguments
/// * `session_id` - A reference to the session ID.
/// * `n` - The number of last chats to retrieve.
///
/// # Returns
/// A vector of chats or an error if the retrieval fails.
async fn get_last_chats(session_id: &str, n: usize) -> Result<Vec<String>, Box<dyn Error>> {
    match DB_INSTANCE.get_last_n_chats(session_id, n) {
        Ok(chats) => Ok(chats),
        Err(e) => {
            error!("Failed to get last chats: {}", e);
            Err(e)
        }
    }
}

/// Generates embeddings for a given prompt and measures the time taken.
///
/// # Arguments
/// * `prompt` - The text prompt to generate embeddings for.
///
/// # Returns
/// A tuple containing the embeddings and the duration, or an error if embedding generation fails.
async fn generate_prompt_embeddings(prompt: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    let (embeddings_result, duration) = measure_time_async(|| generate_text_embedding(prompt)).await;

    match embeddings_result {
        Ok(embeddings_value) => {
            debug!("Time elapsed in generating embeddings: {:?}", duration);
            Ok(embeddings_value)
        }
        Err(e) => {
            error!("Failed to generate embeddings: {}", e);
            Err(e)
        }
    }
}

/// Queries the nearest embeddings based on the generated embeddings.
///
/// # Arguments
/// * `embeddings` - The embeddings to query.
/// * `limit` - The number of nearest embeddings to retrieve.
///
/// # Returns
/// A vector of tuples (rowid, distance, prompt, compressed_prompt_response), or an error.
async fn query_nearest_embeddings(embeddings: Vec<f32>, limit: usize) -> Result<Vec<(i64, f64, String, String)>, Box<dyn Error>> {
    match DB_INSTANCE.query_nearest_embeddings(embeddings.clone(), limit) {
        Ok(context) => Ok(context),
        Err(e) => {
            error!("Failed to query nearest embeddings: {}", e);
            Err(e.into())
        }
    }
}

/// Queries the session context based on the embeddings.
///
/// # Arguments
/// * `embeddings` - The embeddings to query.
/// * `limit` - The number of session context items to retrieve.
///
/// # Returns
/// A vector of tuples (file_path, chunk_type, content), or an error.
async fn query_session_context(embeddings: Vec<f32>, limit: usize) -> Result<Vec<(String, String, String)>, Box<dyn Error>> {
    match DB_INSTANCE.query_session_context(embeddings, limit) {
        Ok(context) => Ok(context),
        Err(e) => {
            error!("Failed to query session context: {}", e);
            Err(e.into())
        }
    }
}

/// Combines and formats the context (last chats, formatted session context, and nearest queries).
///
/// # Arguments
/// * `last_chats` - The vector of last chats.
/// * `rag_context` - The session context (file path, content, etc.).
/// * `query_context` - The nearest embeddings queries.
///
/// # Returns
/// A formatted string combining the context.
fn combine_contexts(last_chats: Vec<String>, rag_context: Vec<(String, String, String)>, query_context: Vec<(i64, f64, String, String)>) -> HashSet<String> {
    let formatted_context: Vec<String> = rag_context
        .iter()
        .map(|(file_path, _, content)| format!("file_path: {}\nContent: {}", file_path, content))
        .collect();

    let nearest_queries: Vec<String> = query_context
        .iter()
        .map(|(_, _, _, compressed_prompt_response)| compressed_prompt_response.clone())
        .collect();

    let mut all_context: HashSet<String> = last_chats.into_iter().collect();  // Remove duplicates
    all_context.extend(formatted_context);
    all_context.extend(nearest_queries);
    
    all_context
}

/// Filters and returns the top `n` reranked documents with positive scores.
///
/// # Arguments
/// * `prompt` - The original prompt.
/// * `all_context` - The combined context.
/// * `top_n` - The number of top documents to return.
///
/// # Returns
/// A formatted string of the top `n` documents or an empty string if none are available.
async fn filter_reranked_documents(prompt: &str, all_context: Vec<String>, top_n: usize) -> String {
    let reranked_documents = rerank_documents(prompt, all_context).await;

    reranked_documents.map_or_else(
        |_| String::new(),
        |docs| {
            docs.into_iter()
                .filter(|(_, _, score)| *score >= 0.0)  // Filter by positive score
                .take(top_n)                            // Take only top N
                .map(|(document, _, _)| document)       // Extract document
                .collect::<Vec<String>>()               // Collect into Vec<String>
                .join("----------CONTEXT----------\n")  // Join with separator
        }
    )
}

/// The main function to generate the context for a given session.
///
/// # Arguments
/// * `session_id` - The session ID.
/// * `prompt` - The user prompt.
/// * `top_n` - The number of top documents to include in the final context.
///
/// # Returns
/// The full context string or an error.
pub async fn make_context(session_id: &str, prompt: &str, top_n: usize) -> Result<String, Box<dyn Error>> {
    let last_chats = get_last_chats(session_id, 4).await?;

    let embeddings = generate_prompt_embeddings(prompt).await?;

    let query_context = query_nearest_embeddings(embeddings.clone(), 10).await?;
    let rag_context = query_session_context(embeddings, 10).await?;

    let all_context_set = combine_contexts(last_chats.clone(), rag_context, query_context);
    let all_context: Vec<String> = all_context_set.into_iter().collect();

    let only_pos_distance_documents = filter_reranked_documents(prompt, all_context, top_n).await;

    let result = if only_pos_distance_documents.is_empty() {
        format!("prior_chat: {}", last_chats.get(0).unwrap_or(&String::new()))
    } else {
        format!("----------CONTEXT----------\n{}\nprior_chat: {}", only_pos_distance_documents, last_chats.get(0).unwrap_or(&String::new()))
    };

    Ok(result)
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