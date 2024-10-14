
use log::{info, error};
use std::pin::Pin;
use bytes::Bytes;

use serde_json::json;
use serde::Deserialize;

use futures::{Stream, StreamExt}; // Ensure StreamExt is imported
use std::error::Error as StdError;  // Importing the correct trait
use reqwest::Error as ReqwestError;
use futures_util::stream::TryStreamExt;
use tokio_stream::wrappers::ReceiverStream;
use crate::utils::{get_llm_temperature, get_local_url};
use crate::platform_variables::get_default_prompt_template;
use reqwest::Client;
use tokio::sync::mpsc;
use futures::stream::unfold;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LLMGenerattionTimings {
    predicted_ms: f64,
    predicted_n: f64,
    predicted_per_second: f64,
    predicted_per_token_ms: f64,
    prompt_ms: f64,
    prompt_n: f64,
    prompt_per_second: f64,
    prompt_per_token_ms: f64,
}


pub async fn local_agent_execution(
    client: &Client,  // Pass the client here
    system_prompt: &str,
    prompt_with_context: &str
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>, Box<dyn StdError + Send + Sync + 'static>> {
    let llm_temperature = get_llm_temperature();
    match local_llm_request(client, system_prompt, prompt_with_context, llm_temperature).await {
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
    client: &Client,  
    system_prompt: &str, 
    prompt_with_context: &str, 
    temperature: f64
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError + Send + Sync + 'static>> {
    
    let llm_server_url = get_local_url();
    send_llm_request(client, &llm_server_url, system_prompt, prompt_with_context, temperature).await
}


async fn send_llm_request(
    client: &Client, 
    llm_server_url: &str, 
    system_prompt: &str, 
    prompt_with_context: &str, 
    temperature: f64
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError + Send + Sync + 'static>> {
    
    let default_prompt_template = get_default_prompt_template();
    
    // Make the full prompt
    let full_prompt = default_prompt_template
        .replace("{system_prompt}", system_prompt)
        .replace("{user_prompt}", prompt_with_context);
    
    info!("{} with temperature {}", full_prompt, temperature);

    let resp = client
        .post(format!("{}/completions", llm_server_url))
        .json(&json!({
            "prompt": full_prompt,
            "stream": true,
            "temperature": temperature,
            "cache_prompt": true
        }))
        .send()
        .await?
        .error_for_status()?;  // Handle HTTP errors automatically

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

    Ok(ReceiverStream::new(rx))
}


pub async fn format_local_llm_response<'a>(
    stream: impl Stream<Item = Result<Bytes, ReqwestError>> + Unpin + 'a,
) -> impl Stream<Item = Result<Bytes, ReqwestError>> + 'a {
    let acc = String::new(); // Initialize accumulator

    unfold((stream, acc), move |(mut stream, acc)| {

        async move {
            if let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                            let  content_to_stream = process_chunk(chunk_str).await;

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
async fn process_chunk(chunk_str: &str) -> String {
    let mut content_to_stream = String::new();

    for line in chunk_str.lines() {
        if line.starts_with("data: ") {
            if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
                if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {

                    content_to_stream.push_str(content);    // Stream content
                }
                if let Some(timings) = json_data.get("timings") {
                    if let Ok(timing_struct) = serde_json::from_value::<LLMGenerattionTimings>(timings.clone()) {
                        let tokens_per_second = calculate_tokens_per_second(
                            timing_struct.predicted_n, 
                            timing_struct.predicted_ms
                        );
                        info!("Tokens generated per second: {:.2}", tokens_per_second);
                    }
                }
            }
        }
    }
    content_to_stream
}

fn calculate_tokens_per_second(predicted_n: f64, predicted_ms: f64) -> f64 {
    let predicted_seconds = predicted_ms / 1000.0;
    predicted_n / predicted_seconds
}