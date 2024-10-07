use actix_web::{ web, HttpResponse, Error };
use async_stream::stream;

use futures::StreamExt; // Ensure StreamExt is imported
use actix_web::Error as ActixError;
use crate::utils::is_cloud_execution_mode;

use std::sync::{ Arc, Mutex };

use super::types::AccumulatedStream;
use super::remote::remote_agent_execution;
use super::local::local_agent_execution;

pub async fn stream_to_chat_client(
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
                        {
                            let mut accumulated = accumulated_content_clone.lock().unwrap();
                            accumulated.push_str(chunk_str);
                        }

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
        .append_header(("X-Session-ID", session_id.clone())) // Add the header here
        .streaming(response_stream);

    Ok(response)
}

pub async fn handle_request(
    system_prompt: &str,
    full_user_prompt: &str
) -> Result<AccumulatedStream, ActixError> {
    let stream: AccumulatedStream = if is_cloud_execution_mode() {
        remote_agent_execution(system_prompt, full_user_prompt).await.map_err(|e|
            ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string()))
        )?
    } else {
        local_agent_execution(system_prompt, full_user_prompt).await.map_err(|e|
            ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string()))
        )?
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
