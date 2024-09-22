use reqwest::Client;
use std::error::Error as StdError;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use serde_json::json;
use regex::Regex;
use bytes::Bytes;
use futures::stream::unfold;
use serde_json::Value;
use crate::request_type::RequestType;
use dotenv::dotenv;
use psutil::process::Process;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::env;
use chrono::Utc;
use std::ops::Div; // Import to use .div()
use actix_web::{error::InternalError, HttpRequest, HttpResponse, Error};
use log::{info, debug, error};
use sysinfo::{System, SystemExt};
use crate::platform_variables::get_default_prompt_template;
use std::process;

// Function to read the CLOUD_EXECUTION_MODE from the environment
pub fn is_cloud_execution_mode() -> bool {
    dotenv().ok(); // Load the .env file if it exists
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}


pub fn get_llm_server_url() -> String {
    let llm_server_url = env::var("LLM_SERVER_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable LLM_SERVER_URL is not set.");
        process::exit(1); // Exit the program with an error code
    });

    llm_server_url
}

pub async fn handle_llm_response(
    req: Option<HttpRequest>,
    system_prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    user_id: &str,
    request_type: RequestType,
) -> Result<HttpResponse, Error> {
    if let Some(req) = req {
        // If the request exists, handle cloud LLM response
        match cloud_llm_response(system_prompt, full_user_prompt, session_id, user_id, request_type).await {
            Ok(stream) => {
                let response = HttpResponse::Ok()
                    .append_header(("X-Session-ID", session_id.to_string()))
                    .streaming(stream);
                return Ok(response);
            }
            Err(e) => {
                return Err(actix_web::error::ErrorInternalServerError(json!({
                    "error": e.to_string()
                })));
            }
        }
    } else {
        // Handle local LLM response if request is not present
        debug!("Local llm being executed with session_id {} and user_id {}", session_id, user_id);

        match local_llm_response(system_prompt,
                        full_user_prompt, 
                        0.2, 
                        session_id, 
                        user_id, 
                        request_type).await {
            Ok(stream) => {
                let response = HttpResponse::Ok()
                    .append_header(("X-Session-ID", session_id.to_string()))
                    .streaming(stream);
                return Ok(response);
            }
            Err(e) => {
                error!("Local llm being executed with session_id {} and user_id {}", session_id, user_id);

                return Err(actix_web::error::ErrorInternalServerError(json!({
                    "error": e.to_string()
                })));
            }
        }
    }
}

async fn local_llm_response(
    system_prompt: &str,
    full_user_prompt: &str,
    temperature: f64,
    session_id: &str,
    user_id: &str,
    request_type: RequestType
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError>> {
    let client = Client::new();
    debug!("Pinging Local LLM server");
    let default_prompt_template = get_default_prompt_template();
    
    //This makes the full prompt by taking the default_prompt_template that
    //depends on the LLM being used
    let full_prompt = default_prompt_template
        .replace("{system_prompt}", system_prompt)
        .replace("{user_prompt}", full_user_prompt);

    let llm_server_url =  get_llm_server_url();

    let resp = client
        .post(&format!("{}/completions", llm_server_url))
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

    // Spawn a new task to handle the streaming of the response
    tokio::spawn(async move {
        let mut stream = resp.bytes_stream();

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

    // Return the receiver as a stream of bytes
    Ok(ReceiverStream::new(rx))
}


async fn cloud_llm_response(
    system_prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    user_id: &str,
    request_type: RequestType,
) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, Box<dyn StdError>> {
    let api_url = "https://api.together.xyz/v1/chat/completions";
    let api_key = std::env::var("TOGETHER_API_KEY")?;  // Fetch the API key from env variables

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



pub async fn format_llmcpp_response(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    user_prompt: &str,
    session_id: &str,
    user_id: &str,
) -> impl Stream<Item = String> {
    let full_response = String::new();

    unfold((stream, full_response), |(mut stream, mut full_response)| async move {
        if let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // Attempt to convert bytes to a UTF-8 string
                    match std::str::from_utf8(&chunk) {
                        Ok(chunk_str) => {
                            // Process each line in the chunk
                            for line in chunk_str.lines() {
                                if line.starts_with("data: ") {
                                    // Extract JSON from the line (skipping "data: ")
                                    if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
                                        // Check for "content" and "stop" flags
                                        if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {
                                            full_response.push_str(content);

                                            // If "stop" flag is found, return the full response and end stream
                                            if json_data.get("stop").is_some() {
                                                return Some((full_response.clone(), (stream, full_response)));
                                            }
                                            // Return the current content and continue
                                            return Some((content.to_string(), (stream, full_response)));
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("Failed to parse chunk as UTF-8");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving chunk: {}", e);
                    return None;
                }
            }
        }

        // End the stream when no more chunks are available
        None
    })
}


pub fn calculate_cpu_usage(pid: u32, interval: Option<u64>) -> f64 {
    // Create a new process object
    let process = Process::new(pid).unwrap();

    // First snapshot of CPU times
    let cpu_times_1 = process.cpu_times().unwrap();
    let time_1 = Instant::now();

    // Wait for the provided interval, defaulting to 1 second if not provided
    let interval_duration = Duration::from_secs(interval.unwrap_or(1));
    sleep(interval_duration);

    // Second snapshot of CPU times
    let cpu_times_2 = process.cpu_times().unwrap();
    let time_2 = Instant::now();

    // Convert the elapsed time to seconds as a floating-point value
    let elapsed_time = time_2.duration_since(time_1).as_secs_f64();

    // Calculate the deltas between the CPU times
    let user_delta = cpu_times_2.user() - cpu_times_1.user();
    let system_delta = cpu_times_2.system() - cpu_times_1.system();
    let total_cpu_time = user_delta + system_delta;

    // Get the total number of CPUs
    let total_cpus = psutil::cpu::cpu_count();

    // Calculate the CPU usage percentage
    let cpu_usage_percent = ((total_cpu_time.div_f32(elapsed_time as f32)) * 100).div_f32(total_cpus as f32);

    cpu_usage_percent.as_secs_f64()
}

pub fn get_ram_usage(pid: u32) -> f64 {
    // Create a new process object
    let process = Process::new(pid).unwrap();

    // Get the memory info
    let memory_info = process.memory_info().unwrap();

    // Convert the RSS (resident set size) from bytes to MB
    let ram_usage_mb = memory_info.rss() as f64 / 1024.0 / 1024.0;

    ram_usage_mb
}

struct ProcessUsage {
    pid: u32,
    cpu_percentage: f64,
    ram_megabytes: f64,
}
fn replace_multiple_spaces(text: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text, " ").trim().to_string()
}


fn current_timestamp() -> i64 {
    // Get the current UTC time and convert it to a Unix timestamp in seconds
    Utc::now().timestamp()
}


pub fn get_total_ram() -> f64{
    // Create a new System instance
    let mut system = System::new_all();

    // Refresh system information (e.g., RAM, CPU)
    system.refresh_memory();

    // Get total memory in kilobytes (KiB)
    let total_memory = system.total_memory();

    // Convert to megabytes (optional)
    let total_memory_gb = total_memory as f64 / (1024.0 * 1024.0);
    return total_memory_gb
}