use log::debug;
use log::error;
use bytes::Bytes;
use std::pin::Pin;
use reqwest::Client;
use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use futures::stream::unfold;
use async_trait::async_trait;
use futures::{Stream, StreamExt}; // Ensure StreamExt is imported
use std::error::Error as StdError;  // Importing the correct trait
use reqwest::Error as ReqwestError;
use actix_web::Error as ActixError;
use futures_util::stream::TryStreamExt;
use tokio_stream::wrappers::ReceiverStream;
use crate::platform_variables::get_default_prompt_template;
use crate::utils::{get_llm_temperature, is_cloud_execution_mode, get_local_url, get_remote_url, get_cloud_api_key};



pub type AccumulatedStream = Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>;

// // Function to read the CLOUD_EXECUTION_MODE from the environment
// pub fn is_cloud_execution_mode() -> bool {

//     dotenv().ok(); // Load the .env file if it exists
//     let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
//     cloud_mode == "true"
// }


// pub fn get_local_url() -> String {
//     dotenv().ok(); // Load the .env file if it exists
//     env::var("LOCAL_URL").unwrap_or_else(|_| {
//         eprintln!("Error: Environment variable LOCAL_URL is not set.");
//         process::exit(1); // Exit the program with an error code
//     })
// }

// pub fn get_remote_url() -> String {
//     dotenv().ok(); // Load the .env file if it exists
//     env::var("REMOTE_URL").unwrap_or_else(|_| {
//         eprintln!("Error: Environment variable REMOTE_URL is not set.");
//         process::exit(1); // Exit the program with an error code
//     })
// }


// pub fn get_cloud_api_key() -> String {
//     dotenv().ok(); // Load the .env file if it exists
//     env::var("CLOUD_API_KEY").unwrap_or_else(|_| {
//         eprintln!("Error: Environment variable CLOUD_API_KEY is not set.");
//         process::exit(1); // Exit the program with an error code
//     })
// }

// pub fn get_llm_temperature() -> f64 {
//     dotenv().ok(); // Load the .env file if it exists
//     env::var("TEMPERATURE").unwrap_or_else(|_| {
//         eprintln!("Error: Environment variable TEMPERATURE is not set.");
//         process::exit(1); // Exit the program with an error code
//     })
//     .parse::<f64>()
//     .unwrap_or_else(|_|{
//         eprintln!("Error: Failed to parse TEMPERATURE as a float.");
//         process::exit(1); // Exit with an error if parsing fails
//     })   
// }

// // Load the environment variables from a `.env` file
// fn load_env() {
//     let current_dir =  env::current_dir().unwrap();
//     let top_dir = current_dir.parent().unwrap().parent().unwrap();
//     let dotenv_path = top_dir.join(".env");
//     dotenv::from_path(dotenv_path).ok();
// }

#[async_trait]
pub trait Agent: Send + Sync {

    fn get_prompt(&self) -> String {
        let llm_prompt_template = get_default_prompt_template();
        llm_prompt_template
            .replace("{system_prompt}", &self.get_system_prompt())
            .replace("{user_prompt}", &self.get_user_prompt())
    }

    async fn execute(&self) -> Result<AccumulatedStream, ActixError> {
        let _prompt = self.get_prompt();

        let stream: AccumulatedStream = if is_cloud_execution_mode() {
            remote_agent_execution(&self.get_system_prompt(), &self.get_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
        } else {
            local_agent_execution(&self.get_system_prompt(), &self.get_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
        };

        let accumulated_content = Arc::new(Mutex::new(String::new()));
        let accumulated_content_clone = Arc::clone(&accumulated_content);

        let accumulated_stream = stream.inspect(move |chunk_result| {
            if let Ok(chunk) = chunk_result {
                if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                    let mut accumulated = accumulated_content_clone.lock().unwrap();
                    accumulated.push_str(chunk_str);
                }
            }
        });

        Ok(Box::pin(accumulated_stream))
    }

    fn to_string(&self) -> String {
        format!("Agent(name='{}')", self.get_name())
    }

    // Helper methods that concrete types must implement
    fn get_name(&self) -> String;
    fn get_user_prompt(&self) -> String;
    fn get_system_prompt(&self) -> String;
    fn get_prompt_with_context(&self) -> String;
}


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