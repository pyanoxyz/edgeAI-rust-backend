

use std::error::Error;
use crate::database::db_config::DB_INSTANCE;
use std::time::{Duration, Instant};
use std::future::Future;
use crate::embeddings::text_embeddings::generate_text_embedding;
use log::{error, debug, info};
use crate::rerank::rerank::rerank_documents;
use std::collections::HashSet;


pub async fn make_context(session_id: &str, prompt: &str, top_n: usize) -> Result<String, Box<dyn Error>> {
    // Retrieve the last 4 chats
    let last_chats = match DB_INSTANCE.get_last_n_chats(session_id, 4) {
        Ok(chats) => chats,
        Err(e) => {
            error!("Failed to get last chats: {}", e);
            return Err(e);
        }
    };

    // Generate embeddings for the prompt
    let (embeddings_result, duration) = measure_time_async(|| generate_text_embedding(prompt)).await;

    let embeddings = match embeddings_result {
        Ok(embeddings_value) => embeddings_value,
        Err(e) => {
            error!("Failed to generate embeddings: {}", e);
            return Err(e);
        }
    };

    debug!("Time elapsed in generating embeddings {:?}", duration);

    // Query nearest embeddings
    //rowid, distance, prompt, compressed_prompt_response
    let query_context = match DB_INSTANCE.query_nearest_embeddings(embeddings.clone(), 10) {
        Ok(context) => context,
        Err(e) => {
            error!("Failed to query nearest embeddings: {}", e);
            return Err(e);
        }
    };

    // Query session context
    // Vec<file_path, chunk_type, content>
    let rag_context = match DB_INSTANCE.query_session_context(embeddings, 10) {
        Ok(context) => context,
        Err(e) => {
            error!("Failed to query session context: {}", e);
            return Err(e);
        }
    };

    let formatted_context: Vec<String> = rag_context
                                .iter()
                                .map(|(file_path, _, content)| format!("file_path {}\nContent {}", file_path.clone(), content.clone()))
                                .collect();
    //rowid, distance, prompt, compressed_prompt_response
    let nearest_queries: Vec<String> = query_context
                                                    .iter()
                                                    .map(|(_, _, _, compressed_prompt_response)| compressed_prompt_response.clone())
                                                    .collect();




    let mut all_context: HashSet<String> = last_chats.clone().into_iter().collect();  // Convert to HashSet to remove duplicates
    all_context.extend(formatted_context.into_iter());    // Extend with the second Vec
    all_context.extend(nearest_queries.into_iter());      // Extend with the third Vec
    
    let all_context: Vec<String> = all_context.into_iter().collect();  // Convert back to Vec


    let reranked_documents = rerank_documents(prompt, all_context).await;
    info!("Reranked documents {:?}", reranked_documents);

    let only_pos_distance_documents: String = reranked_documents
    .map(|docs| {
        docs.into_iter()
            .filter(|(_, _, score)| *score >= 0.0)  // Filter based on the score (f32)
            .take(top_n)                            // Take only the top N documents
            .map(|(document, _, _)| document)       // Extract the document part of the tuple
            .collect::<Vec<String>>()               // Collect into a Vec<String>
            .join("----------CONTEXT----------\n")  // Join with separators
    })
    .unwrap_or_else(|_| String::new()); 

    info!("only_pos_distance_documents {:?}", only_pos_distance_documents);



    let result = if only_pos_distance_documents.is_empty() {
        info!("only_pos_distance_documents is empty");
        format!("prior_chat: {}", last_chats.get(0).unwrap_or(&String::new()))
    } else {
        info!("only_pos_distance_documents is not empty");
        format!("----------CONTEXT----------\n{}\nprior_chat: {}", only_pos_distance_documents, last_chats.get(0).unwrap_or(&String::new()))

    };
    info!("only_pos_distance_documents {:?}", only_pos_distance_documents);

    // info!("Reranked documents {:?}", rerank_documents);

    // Return the result (replace with actual result processing)
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