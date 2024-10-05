use std::env;
use std::process;
use bytes::Bytes;
use std::sync::Arc;
use dotenv::dotenv;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use serde_json::Value;
use futures::StreamExt;
use log::{ debug, error };
use futures::stream::unfold;
use sysinfo::{ System, SystemExt };
use std::error::Error as StdError;
use reqwest::Error as ReqwestError;
use crate::request_type::RequestType;
use futures_util::stream::TryStreamExt;
use actix_web::{ HttpResponse, Error };
use tokio_stream::{ wrappers::ReceiverStream, Stream };
use crate::platform_variables::get_default_prompt_template;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;

pub fn is_cloud_execution_mode() -> bool {
    load_env(); // Load the .env file from the specified path
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}

pub fn get_local_url() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("LOCAL_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable LOCAL_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_remote_url() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("REMOTE_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable REMOTE_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_cloud_api_key() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("CLOUD_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable CLOUD_API_KEY is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_llm_temperature() -> f64 {
    load_env(); // Load the .env file from the specified path
    env::var("TEMPERATURE")
        .unwrap_or_else(|_| {
            eprintln!("Error: Environment variable TEMPERATURE is not set.");
            process::exit(1); // Exit the program with an error code
        })
        .parse::<f64>()
        .unwrap_or_else(|_| {
            eprintln!("Error: Failed to parse TEMPERATURE as a float.");
            process::exit(1); // Exit with an error if parsing fails
        })
}
// Load the environment variables from a `.env` file
fn load_env() {
    let current_dir =  env::current_dir().unwrap();
    let top_dir = current_dir.parent().unwrap();
    let dotenv_path = top_dir.join(".env");
    dotenv::from_path(dotenv_path).ok();
}

pub async fn local_llm_response(
    system_prompt: &str,
    prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    user_id: &str,
    request_type: RequestType
) -> Result<HttpResponse, Error> {
    match local_llm_request(system_prompt, full_user_prompt, 0.2).await {
        Ok(stream) => {
            let prompt_owned = Arc::new(prompt.to_owned());
            let session_id_owned = Arc::new(session_id.to_owned());
            let user_id_owned = Arc::new(user_id.to_owned());
            let request_type_owned = Arc::new(request_type.to_string().to_owned()); // Here, `request_type` is moved

            // Clone request_type if you need to use it later
            // let request_type_clone = request_type.clone();

            let formatted_stream = format_local_llm_response(
                stream,
                prompt_owned.clone(), // Clone Arc for shared ownership
                session_id_owned.clone(),
                user_id_owned.clone(),
                request_type_owned.clone()
            ).await;

            let response = HttpResponse::Ok()
                .append_header(("X-Session-ID", session_id.to_string()))
                .streaming(formatted_stream);
            Ok(response)
        }

        Err(e) => {
            error!(
                "Local llm being executed with session_id {} and user_id {} {}",
                session_id,
                user_id,
                e
            );

            Err(
                actix_web::error::ErrorInternalServerError(
                    json!({
            "error": e.to_string()
        })
                )
            )
        }
    }
}

pub async fn remote_llm_response(
    system_prompt: &str,
    _prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    _user_id: &str,
    _request_type: RequestType
) -> Result<HttpResponse, Error> {
    match cloud_llm_response(system_prompt, full_user_prompt).await {
        Ok(stream) => {
            let response = HttpResponse::Ok()
                .append_header(("X-Session-ID", session_id.to_string()))
                .streaming(stream);
            Ok(response)
        }
        Err(e) => {
            Err(
                actix_web::error::ErrorInternalServerError(
                    json!({
                "error": e.to_string()
            })
                )
            )
        }
    }
}

async fn local_llm_request(
    system_prompt: &str,
    full_user_prompt: &str,
    temperature: f64
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError>> {
    let client = Client::new();
    let default_prompt_template = get_default_prompt_template();

    //This makes the full prompt by taking the default_prompt_template that
    //depends on the LLM being used
    let full_prompt = default_prompt_template
        .replace("{system_prompt}", system_prompt)
        .replace("{user_prompt}", full_user_prompt);

    let llm_server_url = get_local_url();
    debug!("{}", full_prompt);

    let resp = client
        .post(format!("{}/completions", llm_server_url))
        .json(
            &json!({
            "prompt": full_prompt,
            "stream": true,
            "temperature": temperature,
            "cache_prompt": true
        })
        )
        .send().await?
        .error_for_status()?; // Handle HTTP errors automatically

    // Create a channel for streaming the response
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut stream = resp.bytes_stream();

        while let Ok(Some(bytes)) = stream.try_next().await {
            if tx.send(Ok(bytes)).await.is_err() {
                eprintln!("Receiver dropped");
                break;
            }
        }

        if let Err(e) = stream.try_next().await {
            let _ = tx.send(Err(e)).await;
        }
    });

    // Return the receiver as a stream of bytes
    Ok(ReceiverStream::new(rx))
}

async fn cloud_llm_response(
    system_prompt: &str,
    full_user_prompt: &str
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError>> {
    let api_url = get_remote_url();

    let api_key = get_cloud_api_key();
    // Prepare the dynamic JSON body for the request
    let request_body =
        json!({
        "model": "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": full_user_prompt
            }
        ],
    });

    // Create a new reqwest client
    let client = Client::new();

    // Make the POST request
    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send().await?
        .error_for_status()?; // Handle any HTTP errors automatically

    // Create a channel for streaming the response
    let (tx, rx) = mpsc::channel(100);

    // Spawn a new task to handle the streaming of the response
    tokio::spawn(async move {
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    if tx.send(Ok(bytes)).await.is_err() {
                        eprintln!("Receiver dropped");
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    break;
                }
            }
        }
    });

    // Return the receiver as a stream
    Ok(ReceiverStream::new(rx))
}
// pub async fn format_local_llm_response(
//     stream: impl Stream<Item = Result<Bytes, ReqwestError>> + Unpin,
//     user_prompt: Arc<String>,    // Now wrapped in Arc for shared ownership
//     session_id: Arc<String>,     // Wrapped in Arc
//     user_id: Arc<String>,
//     request_type: Arc<String>      // Wrapped in Arc
// ) -> impl Stream<Item = Result<Bytes, ReqwestError>> {
//     let accumulated_content = String::new();

//     unfold((stream, accumulated_content), move |(mut stream, mut acc)| {
//         // The cloning should happen inside the async block
//         let user_id_cloned = Arc::clone(&user_id);
//         let session_id_cloned = Arc::clone(&session_id);
//         let user_prompt_cloned = Arc::clone(&user_prompt);
//         let request_type_cloned = Arc::clone(&request_type);

//         async move {
//             if let Some(chunk_result) = stream.next().await {
//                 match chunk_result {
//                     Ok(chunk) => {
//                         if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
//                             let mut content_to_stream = String::new();
//                             for line in chunk_str.lines() {
//                                 if line.starts_with("data: ") {
//                                     if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
//                                         if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {
//                                             acc.push_str(content); // Accumulate content
//                                             content_to_stream.push_str(content); // Stream content
//                                         }
//                                     }
//                                 }
//                             }

//                             if !content_to_stream.is_empty() {
//                                 // Stream the content that was extracted
//                                 return Some((Ok(Bytes::from(content_to_stream)), (stream, acc)));
//                             }
//                         } else {
//                             eprintln!("Failed to parse chunk as UTF-8");
//                         }
//                     }
//                     Err(e) => {
//                         eprintln!("Error receiving chunk: {}", e);
//                         return Some((Err(e), (stream, acc)));
//                     }
//                 }
//             } else {
//                 // End of stream, process accumulated content
//                 if !acc.is_empty() {
//                     debug!("Stream has ended: {}", acc);
//                     let result: Result<Vec<String>, anyhow::Error> = get_attention_scores(&acc).await;
//                     let tokens = match result {
//                         Ok(tokens) => tokens,
//                         Err(e) =>  {println!("Error while unwrapping tokens: {:?}", e);
//                         return None
//                     }
//                     };
//                     let embeddings_result = generate_text_embedding(&acc).await;

//                     // Extract embeddings if the result is Ok, otherwise return None
//                     let embeddings = match embeddings_result {
//                         Ok(embeddings) => embeddings,
//                         Err(_) => return None,
//                     };
//                     debug!("{:?}", embeddings);
//                     let compressed_prompt = tokens.join(" ");
//                     debug!("Compressed Prompt {:?}", compressed_prompt);

//                     DB_INSTANCE.store_chats(
//                         &user_id_cloned,
//                         &session_id_cloned,
//                         &user_prompt_cloned,
//                         &compressed_prompt,
//                         &acc,
//                         &embeddings[..],
//                         &request_type_cloned
//                     );

//                 }
//                 return None;
//             }
//             // In case there was no content to stream, continue to the next chunk
//             Some((Ok(Bytes::new()), (stream, acc)))
//         }
//     })
// }

pub async fn format_local_llm_response(
    stream: impl Stream<Item = Result<Bytes, ReqwestError>> + Unpin,
    user_prompt: Arc<String>, // Wrapped in Arc for shared ownership
    session_id: Arc<String>, // Wrapped in Arc
    user_id: Arc<String>,
    request_type: Arc<String> // Wrapped in Arc
) -> impl Stream<Item = Result<Bytes, ReqwestError>> {
    let accumulated_content = String::new();

    unfold((stream, accumulated_content), move |(mut stream, mut acc)| {
        let user_id_cloned = Arc::clone(&user_id);
        let session_id_cloned = Arc::clone(&session_id);
        let user_prompt_cloned = Arc::clone(&user_prompt);
        let request_type_cloned = Arc::clone(&request_type);

        async move {
            if let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                            let (new_acc, content_to_stream) = process_chunk(
                                &chunk_str,
                                &acc
                            ).await;

                            acc = new_acc;
                            if !content_to_stream.is_empty() {
                                return Some((Ok(Bytes::from(content_to_stream)), (stream, acc)));
                            }
                        } else {
                            eprintln!("Failed to parse chunk as UTF-8");
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving chunk: {}", e);
                        return Some((Err(e), (stream, acc)));
                    }
                }
            } else {
                // End of stream, process accumulated content
                if !acc.is_empty() {
                    handle_end_of_stream(
                        &acc,
                        &user_id_cloned,
                        &session_id_cloned,
                        &user_prompt_cloned,
                        &request_type_cloned
                    ).await;
                }
                return None;
            }

            Some((Ok(Bytes::new()), (stream, acc)))
        }
    })
}

/// Process each chunk of the stream, extracting content and accumulating it
async fn process_chunk(chunk_str: &str, acc: &str) -> (String, String) {
    let mut accumulated_content = acc.to_string();
    let mut content_to_stream = String::new();

    for line in chunk_str.lines() {
        if line.starts_with("data: ") {
            if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
                if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {
                    accumulated_content.push_str(content); // Accumulate content
                    content_to_stream.push_str(content); // Stream content
                }
            }
        }
    }

    (accumulated_content, content_to_stream)
}

/// Handle the end of the stream by processing accumulated content
async fn handle_end_of_stream(
    acc: &str,
    _user_id: &Arc<String>,
    _session_id: &Arc<String>,
    _user_prompt: &Arc<String>,
    _request_type: &Arc<String>
) {
    debug!("Stream has ended: {}", acc);

    let result= get_attention_scores(&acc).await;
    let tokens = match result {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("Error while unwrapping tokens: {:?}", e);
            return;
        }
    };

    let embeddings_result = generate_text_embedding(acc).await;
    let _embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => {
            return;
        }
    };

    let compressed_prompt = tokens.join(" ");
    debug!("Compressed Prompt {:?}", compressed_prompt);

    // store_in_db(
    //     user_id,
    //     session_id,
    //     user_prompt,
    //     &compressed_prompt,
    //     acc,
    //     embeddings.as_slice(),
    //     request_type,
    // )
    // .await;
}

pub fn get_total_ram() -> f64 {
    // Create a new System instance
    let mut system = System::new_all();

    // Refresh system information (e.g., RAM, CPU)
    system.refresh_memory();

    // Get total memory in kilobytes (KiB)
    let total_memory = system.total_memory();

    // Convert to megabytes (optional)
    let total_memory_gb = (total_memory as f64) / (1024.0 * 1024.0);
    total_memory_gb
}
