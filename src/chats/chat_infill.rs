use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use crate::llm_stream::local::format_local_llm_response;
use crate::llm_stream::types::AccumulatedStream;
use serde::{ Deserialize, Serialize };
use std::sync::{ Arc, Mutex };
use serde_json::{ json, Value };
use reqwest::Client;
use async_stream::stream;

use log::error;
use std::pin::Pin;
use bytes::Bytes;
use actix_web::Error as ActixError;

use futures::{ Stream, StreamExt }; // Ensure StreamExt is imported
use std::error::Error as StdError; // Importing the correct trait
use reqwest::Error as ReqwestError;
use futures_util::stream::TryStreamExt;
use tokio_stream::wrappers::ReceiverStream;
use crate::utils::get_infill_local_url;
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
pub struct InfillRequest {
    pub code_before: String,
    pub code_after: String,
    pub infill_id: String,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(chat_infill); // Register the correct route handler
}

#[post("/chat/infill")]
pub async fn chat_infill(
    data: web::Json<InfillRequest>,
    client: web::Data<Client>,
    _req: HttpRequest
) -> Result<HttpResponse, Error> {
    let infill_id = &data.infill_id;

    // FIM completion prompt for Qwen2.5 coder
    let infill_prompt = format!(
        r#"<|fim_prefix|>{code_before_cursor}<|fim_suffix|>{code_after_cursor}<|fim_middle|>"#,
        code_before_cursor = &data.code_before,
        code_after_cursor = &data.code_after
    );
    // adjust below keys according to how model is loaded and the type of model is being used
    // settings for model: Qwen2.5 Coder 7b instruct
    let infill_req_body =
        json!({
        "max_tokens": 2048,
        "temperature": 0.8,
        // "t_max_predict_ms": 2500,
        "stream": true,
        "stop": [
            "<|endoftext|>",
            "<|fim_prefix|>",
            "<|fim_middle|>",
            "<|fim_suffix|>",
            "<|fim_pad|>",
            "<|repo_name|>",
            "<|file_sep|>",
            "<|im_start|>",
            "<|im_end|>",
            "\n\n",
            "\r\n\r\n",
            "/src/",
            "#- coding: utf-8",
            "```",
            "\nfunction",
            "\nclass",
            "\nmodule",
            "\nexport",
            "\nimport"
        ],
        "prompt": infill_prompt
    });

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Adding context for the infill will increase generation time

    let response = stream_infill_request(&client, &infill_id, infill_req_body, tx).await?;

    Ok(response)
}

pub async fn stream_infill_request(
    client: &Client, // Pass the client here
    infill_id: &str,
    infill_req_body: Value,
    tx: tokio::sync::oneshot::Sender<()>
) -> Result<HttpResponse, Error> {
    let stream_result = handle_infill_request(client, infill_req_body).await;
    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                "error": format!("Local LLM response error: {}", e)
            })
                )
            );
        }
    };

    // Stream chunks to the client in real-time and accumulate
    let response_stream = stream! {
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                        // Accumulate the content in memory

                        // Yield each chunk to the stream
                        yield Ok::<_, Error>(web::Bytes::from(chunk_str.to_owned()));
                    }
                }
                Err(e) => {
                    yield Err(
                        actix_web::error::ErrorInternalServerError(
                            format!("Error while streaming: {}", e)
                        )
                    );
                }
            }
        }

        // Notify that streaming is complete
        let _ = tx.send(());
    };

    // Return the response as a streaming body
    let response = HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("X-INFILL-ID", infill_id)) // Add the header here
        .streaming(response_stream);

    Ok(response)
}

pub async fn infill_agent_execution(
    client: &Client,
    infill_req_body: Value
) -> Result<
    Pin<Box<dyn Stream<Item = Result<Bytes, ReqwestError>> + Send>>,
    Box<dyn StdError + Send + Sync + 'static>
> {
    match send_llm_request(client, infill_req_body).await {
        Ok(stream) => {
            let formatted_stream = format_local_llm_response(stream).await;
            Ok(Box::pin(formatted_stream)) // Pin the stream here using Box::pin
        }
        Err(e) => {
            error!("Infill execution error: {}", e);
            Err(e.into()) // Use `into()` to convert the error directly into `Box<dyn StdError>`
        }
    }
}

pub async fn handle_infill_request(
    client: &Client,
    infill_req_body: Value
) -> Result<AccumulatedStream, ActixError> {
    let stream: AccumulatedStream = infill_agent_execution(client, infill_req_body).await.map_err(
        |e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string()))
    )?;
    // Shared state using Arc<Mutex<_>>
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);
    // Apply inspect on the stream to accumulate content
    let accumulated_stream = stream.inspect(move |chunk_result| {
        if let Ok(chunk) = chunk_result {
            if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                let mut accumulated = accumulated_content_clone.lock().unwrap();
                accumulated.push_str(chunk_str);
            }
        }
    });
    // Return the stream directly wrapped in a Pin
    Ok(Box::pin(accumulated_stream))
}

pub async fn send_llm_request(
    client: &Client,
    infill_req_body: Value
) -> Result<
    impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
    Box<dyn StdError + Send + Sync + 'static>
> {
    let llm_server_url = get_infill_local_url();

    let resp = client
        .post(format!("{}/completion", llm_server_url))
        .json(&infill_req_body)
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

    Ok(ReceiverStream::new(rx))
}
