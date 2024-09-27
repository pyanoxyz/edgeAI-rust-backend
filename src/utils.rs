use reqwest::Client;
use std::error::Error as StdError;
use tokio::sync::mpsc;
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
use actix_web::{HttpRequest, HttpResponse, Error};
use log::{debug, error};
use sysinfo::{System, SystemExt};
use crate::platform_variables::get_default_prompt_template;
use std::process;
use futures_util::stream::TryStreamExt;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use futures::StreamExt;
use reqwest::Error as ReqwestError;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
use crate::database::db_config::DB_INSTANCE;
use std::sync::Arc;

// Function to read the CLOUD_EXECUTION_MODE from the environment
pub fn is_cloud_execution_mode() -> bool {
    dotenv().ok(); // Load the .env file if it exists
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}


pub fn get_llm_server_url() -> String {
    

    env::var("LLM_SERVER_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable LLM_SERVER_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}


pub async fn local_llm_response(
    system_prompt: &str,
    prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    user_id: &str,
    request_type: RequestType,
) -> Result<HttpResponse, Error> {

    match local_llm_request(system_prompt,
        full_user_prompt, 
        0.2).await {
            Ok(stream) => {
            let prompt_owned = Arc::new(prompt.to_owned());
            let session_id_owned = Arc::new(session_id.to_owned());
            let user_id_owned = Arc::new(user_id.to_owned());
            let request_type_owned = Arc::new(request_type.to_string().to_owned());  // Here, `request_type` is moved

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
        error!("Local llm being executed with session_id {} and user_id {}", session_id, user_id);

        Err(actix_web::error::ErrorInternalServerError(json!({
            "error": e.to_string()
        })))
        }
}

}

pub async fn remote_llm_response(
    system_prompt: &str,
    prompt: &str,
    full_user_prompt: &str,
    session_id: &str,
    user_id: &str,
    request_type: RequestType,
) -> Result<HttpResponse, Error> {
    match cloud_llm_response(system_prompt, full_user_prompt).await {
        Ok(stream) => {
            let response = HttpResponse::Ok()
                .append_header(("X-Session-ID", session_id.to_string()))
                .streaming(stream);
            Ok(response)
        }
        Err(e) => {
            Err(actix_web::error::ErrorInternalServerError(json!({
                "error": e.to_string()
            })))
        }
    }
}


async fn local_llm_request(
    system_prompt: &str,
    full_user_prompt: &str,
    temperature: f64,

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
    full_user_prompt: &str,
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
pub async fn format_local_llm_response(
    stream: impl Stream<Item = Result<Bytes, ReqwestError>> + Unpin,
    user_prompt: Arc<String>,    // Now wrapped in Arc for shared ownership
    session_id: Arc<String>,     // Wrapped in Arc
    user_id: Arc<String>,
    request_type: Arc<String>      // Wrapped in Arc
) -> impl Stream<Item = Result<Bytes, ReqwestError>> {
    let accumulated_content = String::new();

    unfold((stream, accumulated_content), move |(mut stream, mut acc)| {
        // The cloning should happen inside the async block
        let user_id_cloned = Arc::clone(&user_id);
        let session_id_cloned = Arc::clone(&session_id);
        let user_prompt_cloned = Arc::clone(&user_prompt);
        let request_type_cloned = Arc::clone(&request_type);

        async move {
            if let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                            let mut content_to_stream = String::new();
                            for line in chunk_str.lines() {
                                if line.starts_with("data: ") {
                                    if let Ok(json_data) = serde_json::from_str::<Value>(&line[6..]) {
                                        if let Some(content) = json_data.get("content").and_then(|c| c.as_str()) {
                                            acc.push_str(content); // Accumulate content
                                            content_to_stream.push_str(content); // Stream content
                                        }
                                    }
                                }
                            }

                            if !content_to_stream.is_empty() {
                                // Stream the content that was extracted
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
                    debug!("Stream has ended: {}", acc);
                    let result: Result<Vec<String>, anyhow::Error> = get_attention_scores(&acc).await;
                    let tokens = match result {
                        Ok(tokens) => tokens,
                        Err(e) =>  {println!("Error while unwrapping tokens: {:?}", e);
                        return None
                    }
                    };
                    let embeddings_result = generate_text_embedding(&acc).await;
                    
                    // Extract embeddings if the result is Ok, otherwise return None
                    let embeddings = match embeddings_result {
                        Ok(embeddings) => embeddings,
                        Err(_) => return None,
                    };
                    debug!("{:?}", embeddings);
                    let compressed_prompt = tokens.join(" ");
                    debug!("Compressed Prompt {:?}", compressed_prompt);

                    DB_INSTANCE.store_chats(
                        &user_id_cloned, 
                        &session_id_cloned, 
                        &user_prompt_cloned, 
                        &compressed_prompt, 
                        &acc, 
                        &embeddings[..],
                        &request_type_cloned
                    );

                }
                return None;
            }
            // In case there was no content to stream, continue to the next chunk
            Some((Ok(Bytes::new()), (stream, acc)))
        }
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
    

    memory_info.rss() as f64 / 1024.0 / 1024.0
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
    total_memory_gb
}