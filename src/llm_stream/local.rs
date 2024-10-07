
use log::{debug, error};
use std::pin::Pin;
use bytes::Bytes;

use serde_json::json;

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


// pub type AccumulatedStream = Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>;

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