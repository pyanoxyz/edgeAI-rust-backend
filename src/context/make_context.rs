

use std::error::Error;
use crate::database::db_config::DB_INSTANCE;
use std::time::{Duration, Instant};
use std::future::Future;
use crate::embeddings::text_embeddings::generate_text_embedding;
use log::{error, info};
use crate::rerank::rerank::rerank_documents;
use std::collections::HashSet;
use crate::similarity_index::index::search_index;


/// Retrieves the last `n` chats for a given session.
///
/// # Arguments
/// * `session_id` - A reference to the session ID.
/// * `n` - The number of last chats to retrieve.
///
/// # Returns
/// A vector of chats or an error if the retrieval fails.
async fn get_last_chats(session_id: &str, n: usize) -> Result<Vec<String>, Box<dyn Error>> {
    let (chats, duration) = measure_time_async(|| async {
        DB_INSTANCE.get_last_n_chats(session_id, n)
    }).await;

    match chats {
        Ok(chats) => {
            info!("Time elapsed in getting last {} chats: {:?}", n, duration);
            Ok(chats)
        }
        Err(e) => {
            error!("Failed to generate embeddings: {}", e);
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
            info!("Time elapsed in generating embeddings: {:?}", duration);
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
async fn query_nearest_chat_embeddings(embeddings: Vec<f32>, limit: usize) -> Result<Vec<(i64, f64, String, String, String)>, Box<dyn Error>> {

    let (chats, duration) = measure_time_async(|| async {
        DB_INSTANCE.query_nearest_embeddings(embeddings.clone(), limit)
    }).await;

    match chats {
        Ok(chats) => {
            info!("Time elapsed in getting last {} nearest embeddings to query: {:?} and got {} nearest embeddings", limit, duration, chats.len());
            Ok(chats)
        }
        Err(e) => {
            error!("Failed to generate embeddings: {}", e);
            Err(e)
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
async fn query_session_context(session_id: &str, embeddings: Vec<f32>, limit: usize) -> Result<Vec<(String, String, String, String)>, Box<dyn Error>> {
    // match DB_INSTANCE.query_session_context(embeddings, limit) {
    //     Ok(context) => {
    //         info!("Nearest embeddings from the database {:?}", context);
    //         Ok(context)},
    //     Err(e) => {
    //         error!("Failed to query session context: {}", e);
    //         Err(e.into())
    //     }
    // }

    // let (chats, duration) = measure_time_async(|| async {
    //     DB_INSTANCE.query_session_context(embeddings, limit)
    // }).await;

    // match chats {
    //     Ok(chats) => {
    //         info!("Time elapsed in getting last {} nearest rag embeddings to query: {:?} and got {} nearest rag embeddings", limit, duration, chats.len());
    //         Ok(chats)
    //     }
    //     Err(e) => {
    //         error!("Failed to generate embeddings: {}", e);
    //         Err(e)
    //     }
    // }

    let chunk_ids = search_index(session_id, embeddings, limit);

    let entries = DB_INSTANCE.get_row_ids(chunk_ids).unwrap();
    Ok(entries)


}

/// Combines and formats the context (last chats, formatted session context, and nearest queries).
///
/// # Arguments
/// * `last_chats` - The vector of last chats.
/// * `rag_context` - The session context (file path, content, etc.).
/// * `query_context` - The nearest embeddings queries.
///
///
/// # Returns
/// A formatted string combining the context.
fn combine_contexts(last_chats: Vec<String>, rag_context: Vec<(String, String, String, String)>, query_context: Vec<(i64, f64, String, String, String)>) -> HashSet<String> {
    // file_path, chunk_type, content, session_id
    let formatted_context: Vec<String> = rag_context
        .iter()
        .map(|(file_path, _, content, _)| format!("file_path: {}\nContent: {}", file_path, content))
        .collect();

    info!("Context from the files {:?}", formatted_context);

    let nearest_queries: Vec<String> = query_context
        .iter()
        .map(|(_, _, _, compressed_prompt_response, _)| compressed_prompt_response.clone())
        .collect();

    info!("Context from the chat history {:?}", nearest_queries);

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
async fn filter_reranked_documents(prompt: &str, all_context: Vec<String>, top_n: usize) -> Result<String, Box<dyn Error>> {
    // info!("RERANKED DOcuments process started");

    // let reranked_documents = rerank_documents(prompt, all_context).await;
    // info!("RERANKED DOcuments {:?}", reranked_documents);

    let (documents, duration) = measure_time_async(|| async {
        rerank_documents(prompt, all_context).await
    }).await;

    info!("Rerank docs resulting length {:?}", documents);
    match documents {
        Ok(docs) => {
            info!("Time elapsed in re ranking documents {:?}", duration);
            info!("Rerank docs resulting length {:?}", docs.len());

            let formatted_docs = docs.into_iter()
            .take(top_n)                            // Take only top N
            .map(|(document, _, _)| document)       // Extract document
            .collect::<Vec<String>>()               // Collect into Vec<String>
            .join("----------CONTEXT----------\n"); // Join with separator

        // Return the formatted string or an empty string if no documents are available
        Ok(if formatted_docs.is_empty() { String::new() } else { formatted_docs })
        }
        Err(e) => {
            error!("Failed to rerank docs: {:?}", e);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))
        }
    }
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

    //SQLITE vector embeddings doesnt support anyother colums execptet tor_id and embeddings
    // as a result we are fetching around 100 nearest do cuments in all the user history
    //and then filtering on the basis of the session_id
    let query_context = query_nearest_chat_embeddings(embeddings.clone(), 100).await?
                                                            .into_iter()
                                                            .filter(|(_, _, _, _, sid)| sid == session_id)
                                                            .collect::<Vec<_>>();
    let rag_context = query_session_context(session_id, embeddings, 10).await?;
                                                        
    let all_context_set = combine_contexts(last_chats.clone(), rag_context, query_context);
    let all_context: Vec<String> = all_context_set.into_iter().collect();

    let only_pos_distance_documents = filter_reranked_documents(prompt, all_context, top_n).await?;
    info!("Reranked documents {:?}", only_pos_distance_documents);

    let result = if only_pos_distance_documents.is_empty() {
        format!("prior_chat: {}", last_chats.get(0).unwrap_or(&String::new()))
    } else {
        format!("----------CONTEXT----------\n{}\nprior_chat: {}", only_pos_distance_documents, last_chats.get(0).unwrap_or(&String::new()))
    };
    info!("Context being fed {}", result);

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