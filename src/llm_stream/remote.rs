

use log::error;
use std::pin::Pin;
use bytes::Bytes;

use serde_json::json;

use futures::{Stream, StreamExt}; // Ensure StreamExt is imported
use std::error::Error as StdError;  // Importing the correct trait
use reqwest::Error as ReqwestError;
use tokio_stream::wrappers::ReceiverStream;
use reqwest::Client;
use tokio::sync::mpsc;
use crate::utils::{get_remote_url, get_cloud_api_key};


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

