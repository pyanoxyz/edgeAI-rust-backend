use log::{debug, error};
use std::pin::Pin;
use bytes::Bytes;

use serde_json::json;
use actix_web::{web, HttpResponse, Error};
use async_stream::stream;

use futures::{Stream, StreamExt}; // Ensure StreamExt is imported
use std::error::Error as StdError;  // Importing the correct trait
use reqwest::Error as ReqwestError;
use actix_web::Error as ActixError;
use futures_util::stream::TryStreamExt;
use tokio_stream::wrappers::ReceiverStream;
use crate::utils::{get_llm_temperature, is_cloud_execution_mode, get_local_url, get_remote_url, get_cloud_api_key};
use crate::platform_variables::get_default_prompt_template;
use reqwest::Client;
use tokio::sync::mpsc;
use futures::stream::unfold;
use serde_json::Value;
use std::sync::{Arc, Mutex};


pub type AccumulatedStream = Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>;



#[derive(Debug)]
pub struct RefactorPrompt {
    pub system_prompt: &'static str,
    pub user_prompt_template: &'static str,
}

impl RefactorPrompt {
    pub fn new(system_prompt: &'static str, user_prompt_template: &'static str) -> Self {
        RefactorPrompt {
            system_prompt,
            user_prompt_template,
        }
    }
}

pub async fn stream_to_client(
    session_id: &str,
    system_prompt: &str,
    full_user_prompt: &str,
    accumulated_content_clone: Arc<Mutex<String>>,
    tx: tokio::sync::oneshot::Sender<()>
) -> Result<HttpResponse, Error> {
    let stream_result = handle_request(system_prompt, full_user_prompt).await;
    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Local LLM response error: {}", e)
            })));
        }
    };

    // Stream chunks to the client in real-time and accumulate
    let response_stream = stream! {
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                        // Accumulate the content in memory
                        {
                            let mut accumulated = accumulated_content_clone.lock().unwrap();
                            accumulated.push_str(chunk_str);
                        }

                        // Yield each chunk to the stream
                        yield Ok::<_, Error>(web::Bytes::from(chunk_str.to_owned()));
                    }
                }
                Err(e) => {
                    yield Err(actix_web::error::ErrorInternalServerError(format!(
                        "Error while streaming: {}",
                        e
                    )));
                }
            }
        }

        // Notify that streaming is complete
        let _ = tx.send(());
    };

    // Return the response as a streaming body
    let response = HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("session_id", session_id.clone())) // Add the header here
        .streaming(response_stream);

    Ok(response)
}

pub async fn handle_request(
    system_prompt: &str,
    full_user_prompt: &str,
) -> Result<AccumulatedStream, ActixError> {
    let stream: AccumulatedStream = if is_cloud_execution_mode() {
        remote_agent_execution(system_prompt, full_user_prompt)
            .await
            .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
    } else {
        local_agent_execution(system_prompt, full_user_prompt)
            .await
            .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
    };

    // Shared state using Arc<Mutex<_>>
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    // Apply inspect on the stream
    let accumulated_stream = stream.inspect(move |chunk_result| {
        if let Ok(chunk) = chunk_result {
            if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                let mut accumulated = accumulated_content_clone.lock().unwrap();
                accumulated.push_str(chunk_str);
            }
        }
    });

    // Since we cannot clone the stream, return the stream directly wrapped in a Pin
    Ok(Box::pin(accumulated_stream))
}

// // TODO try adding this dynamically to all prompts
// const FORMATTING_PROMPT: &str =
//     r#"
//     For formatting:
//     - Use Gfm if necessary
//     - Use proper tabs spaces and indentation.
//     - Use single-line code blocks with `<code here>`.
//     - Use comments syntax of the programming language for comments in code blocks.
//     - Use multi-line blocks with:
//     ```<language>
//     <code here>
//     ```
//     "#;

pub async fn local_agent_execution(
    system_prompt: &str,
    prompt_with_context: &str
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>, Box<dyn StdError + Send + Sync + 'static>> {
    let llm_temperature = get_llm_temperature();
    match local_llm_request(system_prompt, prompt_with_context, llm_temperature).await {
        Ok(stream) => {
            let formatted_stream = format_local_llm_response(stream).await;
            Ok(Box::pin(formatted_stream)) // Pin the stream here using Box::pin
        }
        Err(e) => {
            error!("Local LLM execution error in Pair programmer: {}", e);
            Err(e.into())  // Use `into()` to convert the error directly into `Box<dyn StdError>`
        }
    }
}

async fn local_llm_request(
    system_prompt: &str,
    prompt_with_context: &str,
    temperature: f64,

) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError + Send + Sync + 'static>> {
    let client = Client::new();
    let default_prompt_template = get_default_prompt_template();
    
    //This makes the full prompt by taking the default_prompt_template that
    //depends on the LLM being used
    let full_prompt = default_prompt_template
        .replace("{system_prompt}", system_prompt)
        .replace("{user_prompt}", prompt_with_context);

    let llm_server_url =  get_local_url();
    debug!("{} with temperature {}", full_prompt, temperature);

    let resp = client
        .post(format!("{}/completions",  llm_server_url))
        .json(&json!({
            "prompt": full_prompt,
            "stream": true,
            "temperature": temperature,
            "cache_prompt": true
        }))
        .send()
        .await?
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
    prompt_with_context: &str,
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>,Box<dyn StdError + Send + Sync + 'static>> {
    let api_url =  get_remote_url();

    let api_key = get_cloud_api_key();
    // Prepare the dynamic JSON body for the request
    let request_body = json!({
        "model": "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": prompt_with_context
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
        .send()
        .await?
        .error_for_status()?;  // Handle any HTTP errors automatically

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



pub async fn format_local_llm_response<'a>(
    stream: impl Stream<Item = Result<Bytes, ReqwestError>> + Unpin + 'a,
) -> impl Stream<Item = Result<Bytes, ReqwestError>> + 'a {
    let acc = String::new(); // Initialize accumulator

    unfold((stream, acc), move |(mut stream, mut acc)| {

        async move {
            if let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                            let  content_to_stream = process_chunk(chunk_str, &acc).await;

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
                // End of stream, mark the stream as done
                return None; // Stream is done
            }

            Some((Ok(Bytes::new()), (stream, acc)))
        }
    })
}
/// Process each chunk of the stream, extracting content and accumulating it
async fn process_chunk(chunk_str: &str, acc: &str) -> String {
    let mut content_to_stream = String::new();

    for line in chunk_str.lines() {
        if line.starts_with("data: ") {
            if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
                if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {
                    content_to_stream.push_str(content);    // Stream content
                }
            }
        }
    }

    content_to_stream
}
pub async fn remote_agent_execution(
    system_prompt: &str,
    prompt_with_context: &str,
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>, Box<dyn StdError + Send + Sync + 'static>> {
    match cloud_llm_response(system_prompt, prompt_with_context).await {
        Ok(stream) => {
            Ok(Box::pin(stream)) // Pin the stream here using Box::pin
        }
        Err(e) => {
            error!("Remote agent execution error in Pair programmer: {}", e);
            Err(e.into())  // Use `into()` to convert the error directly into `Box<dyn StdError>`
        }
    }
}

