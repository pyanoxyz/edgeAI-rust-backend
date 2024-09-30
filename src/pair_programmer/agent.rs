use async_trait::async_trait;
use dotenv::dotenv;
use log::debug;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::sync::Arc;
use crate::platform_variables::get_default_prompt_template;
// use std::error::Error as StdError;
use tokio::sync::mpsc;
use bytes::Bytes;
use futures::stream::unfold;
use serde_json::Value;
use actix_web::{error::ErrorInternalServerError, error::ResponseError, HttpResponse,  Error as ActixError};
use log::{error};
use anyhow::anyhow;
use thiserror::Error;
use std::fmt;
use std::error::Error as StdError;  // Importing the correct trait
use anyhow::Error as AnyhowError;  // Import anyhow's Error type

use std::process;
use futures_util::stream::TryStreamExt;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use futures::StreamExt;
use reqwest::Error as ReqwestError;
use regex::Regex;
 // Using anyhow for error handling
// use reqwest::Error as ReqwestError;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
use crate::database::db_config::DB_INSTANCE;
use crate::pair_programmer::pair_programmer_types::Step;

use super::pair_programmer_api;

// Custom logger would need to be implemented for logging
// Define your logger similar to the python logger if needed



fn parse_steps(input: &str) -> Vec<Step> {
    // Updated regex to capture step number and heading
    let re_step = Regex::new(r"(?i)\s*Step\s+(\d+)\s*:\s*(.+)").unwrap(); // Case-insensitive match for "Step X: heading"
    let re_tool = Regex::new(r"(?i)\s*Tool\s*:\s*`([\w\-]+)`\s*").unwrap(); // Handle spaces and allow hyphenated tool names
    let re_action = Regex::new(r#"(?i)\s*Action\s*:\s*`<function=([^>]+)>\s*\{\{(.+?)\}\}\s*</function>`"#).unwrap(); // Handle spaces and missing </function>

    let mut steps = Vec::new();
    let mut current_step_number = 0;
    let mut current_heading = String::new();
    let mut current_tool = String::new();
    let mut current_action = String::new();

    for line in input.lines() {
        let trimmed_line = line.trim(); // Trim leading and trailing whitespace
        if let Some(caps) = re_step.captures(trimmed_line) {
            // Save the previous step before starting a new one
            if current_step_number > 0 {
                steps.push(Step {
                    step_number: current_step_number,
                    heading: current_heading.clone(),
                    tool: current_tool.clone(),
                    action: current_action.clone(),
                });
            }
            // Start a new step, capture step number and heading
            current_step_number = caps[1].parse().unwrap_or(0);
            current_heading = caps[2].to_string();
            current_tool.clear();
            current_action.clear();
        } else if let Some(caps) = re_tool.captures(trimmed_line) {
            current_tool = caps[1].to_string();
        } else if let Some(caps) = re_action.captures(trimmed_line) {
            current_action = format!("<function={}>{{{{{}}}}}", &caps[1], &caps[2]); // Capture full action with lazy matching
        }
    }

    // Add the last step if it exists
    if current_step_number > 0 {
        steps.push(Step {
            step_number: current_step_number,
            heading: current_heading.clone(),
            tool: current_tool.clone(),
            action: current_action.clone(),
        });
    }

    steps
}



// Function to read the CLOUD_EXECUTION_MODE from the environment
pub fn is_cloud_execution_mode() -> bool {

    dotenv().ok(); // Load the .env file if it exists
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}


pub fn get_local_url() -> String {
    dotenv().ok(); // Load the .env file if it exists
    env::var("LOCAL_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable LOCAL_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_remote_url() -> String {
    dotenv().ok(); // Load the .env file if it exists
    env::var("REMOTE_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable REMOTE_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}


pub fn get_cloud_api_key() -> String {
    dotenv().ok(); // Load the .env file if it exists
    env::var("CLOUD_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable CLOUD_API_KEY is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_llm_temperature() -> f64 {
    dotenv().ok(); // Load the .env file if it exists
    env::var("TEMPERATURE").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable TEMPERATURE is not set.");
        process::exit(1); // Exit the program with an error code
    })
    .parse::<f64>()
    .unwrap_or_else(|_|{
        eprintln!("Error: Failed to parse TEMPERATURE as a float.");
        process::exit(1); // Exit with an error if parsing fails
    })   
}

// Load the environment variables from a `.env` file
fn load_env() {
    let current_dir =  env::current_dir().unwrap();
    let top_dir = current_dir.parent().unwrap().parent().unwrap();
    let dotenv_path = top_dir.join(".env");
    dotenv::from_path(dotenv_path).ok();
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn new(user_prompt: String, prompt_with_context: String) -> Self;

    fn get_prompt(&self) -> String {
        let llm_prompt_template = get_default_prompt_template();
        llm_prompt_template
            .replace("{system_prompt}", &self.get_system_prompt())
            .replace("{user_prompt}", &self.get_user_prompt())
    }

    async fn execute(&self, user_id: &str, session_id: &str, pair_programmer_id: &str) -> Result<HttpResponse, ActixError> {
        let prompt = self.get_prompt();

        if is_cloud_execution_mode() {
            // Remote execution when cloud_execution_mode is enabled
            remote_agent_execution(&self.get_system_prompt(), &self.get_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))
        } else {
            // Local execution when running in local mode
            local_agent_execution(
                &self.get_system_prompt(),
                &self.get_user_prompt(),
                &self.get_prompt_with_context(),
                user_id,
                session_id, 
                pair_programmer_id
            )
            .await
            .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))
        }

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
    user_prompt: &str,
    prompt_with_context: &str,
    user_id: &str, 
    session_id: &str,
    pair_programmer_id: &str
) -> Result<HttpResponse, Box<dyn StdError + Send + Sync + 'static>> {
    let llm_temperature = get_llm_temperature();
    match local_llm_request(system_prompt, prompt_with_context, llm_temperature).await {
        Ok(stream) => {
            let prompt_owned = Arc::new(user_prompt.to_owned());
            let user_id_owned = Arc::new(user_id.to_owned());
            let session_id_owned = Arc::new(session_id.to_owned());
            let pair_programmer_owned = Arc::new(pair_programmer_id.to_owned());
            
            let formatted_stream = format_local_llm_response(stream,                 
                                                        Arc::clone(&prompt_owned),
                                                        Arc::clone(&user_id_owned), 
                                                        Arc::clone(&session_id_owned),
                                                        Arc::clone(&pair_programmer_owned) 
                                                    )
                                                        .await;
            let response = HttpResponse::Ok().streaming(formatted_stream);
            Ok(response)
        }
        Err(e) => {
            error!("Local LLM execution error in Pair programmer: {}", e);
            Err(e.into())  // Use `into()` to convert the error directly into `Box<dyn StdError>`
        }
    }
}

pub async fn remote_agent_execution(
    system_prompt: &str,
    prompt_with_context: &str,
) -> Result<HttpResponse, Box<dyn StdError + Send + Sync + 'static>> {
    match cloud_llm_response(system_prompt, prompt_with_context).await {
        Ok(stream) => {
            let response = HttpResponse::Ok().streaming(stream);
            Ok(response)
        }
        Err(e) => {
            error!("Remote agent execution error in Pair programmer: {}", e);
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
    debug!("{}", full_prompt);

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
    user_prompt: Arc<String>,
    user_id: Arc<String>,
    session_id: Arc<String>,
    pair_programmer_id: Arc<String>
) -> impl Stream<Item = Result<Bytes, ReqwestError>> + 'a {
    let accumulated_content = String::new();

    unfold((stream, accumulated_content), move |(mut stream, mut acc)| {
        let user_prompt_cloned = Arc::clone(&user_prompt);
        let user_id_cloned = Arc::clone(&user_id);
        let session_id_cloned = Arc::clone(&session_id);
        let pair_programmer_id_cloned = Arc::clone(&pair_programmer_id);

        async move {
            if let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                            let (new_acc, content_to_stream) = process_chunk(&chunk_str, &acc).await;

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
                        &user_prompt_cloned,
                        &user_id_cloned,
                        &session_id_cloned,
                        &pair_programmer_id_cloned
                    )
                    .await;
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
                    accumulated_content.push_str(content);  // Accumulate content
                    content_to_stream.push_str(content);    // Stream content
                }
            }
        }
    }

    (accumulated_content, content_to_stream)
}

/// Handle the end of the stream by processing accumulated content
/// This user_prompt is the original prompt that user has gave us
/// not the prompt with context because that has already been passed in llm_request
async fn handle_end_of_stream(
    input: &str,
    user_prompt: &Arc<String>,
    user_id: &Arc<String>,
    session_id: &Arc<String>,
    pair_programmer_id: &Arc<String>
) {
    debug!("Stream has ended: {}", input);
    let steps = parse_steps(input);
    for step in &steps {
        println!("{:?}", step);
    }
    DB_INSTANCE.store_new_pair_programming_session(user_id, session_id, pair_programmer_id, user_prompt, &steps); 


    let result: Result<Vec<String>, anyhow::Error> = get_attention_scores(&input).await;
    let tokens = match result {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("Error while unwrapping tokens: {:?}", e);
            return;
        }
    };

    let embeddings_result = generate_text_embedding(input).await;
    let embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => return,
    };

}

/// Store the processed content and embeddings into the database
async fn store_in_db(
    user_id: &Arc<String>,
    session_id: &Arc<String>,
    user_prompt: &Arc<String>,
    compressed_prompt: &str,
    acc: &str,
    embeddings: &[f32],
    request_type: &Arc<String>,
) {
    DB_INSTANCE.store_chats(
        user_id,
        session_id,
        user_prompt,
        compressed_prompt,
        acc,
        embeddings,
        request_type,
    );
}
