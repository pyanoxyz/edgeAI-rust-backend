use uuid::Uuid;
use serde_json::json;
use async_stream::stream;
use actix_web::FromRequest;
// use futures_util::{Stream, StreamExt}; // Import this trait for accessing `.next()`
use std::sync::{Arc, Mutex};
use log::{info, debug, error};
use serde::{Deserialize, Serialize};
use crate::rag::code_rag::index_code;
use crate::similarity_index::index::search_index;
use crate::pair_programmer::agent::Agent;
use crate::database::db_config::DB_INSTANCE;
use crate::pair_programmer::agent_enum::AgentEnum;
use crate::summarization::summarize::summarize_text;
use crate::prompt_compression::compress::get_attention_scores;
use crate::embeddings::text_embeddings::generate_text_embedding;
use actix_web::{post, web, get, HttpRequest, HttpResponse, Error};
use crate::pair_programmer::pair_programmer_utils::{data_validation, rethink_prompt_with_context, parse_steps, parse_step_number, prompt_with_context, prompt_with_context_for_chat };
use futures::StreamExt; // Ensure StreamExt is imported
use crate::session_manager::check_session;
use reqwest::Client;
use super::types::StepData;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateStepsRequest {
    pub task: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub files: Option<Vec<String>>,

}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummarizeChatRequest {
    pub pair_programmer_id: String,
    pub step_number: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RethinkRequest {
    pub pair_programmer_id: String,
    pub step_number: String
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteStepRequest {
    pub pair_programmer_id: String,
    pub step_number: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatStepRequest {
    pub pair_programmer_id: String,
    pub step_number: String,
    pub prompt: String
}



pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(pair_programmer_generate_steps)
    .service(get_steps)
    .service(execute_step)
    .service(chat_step)
    .service(chat_summary)
    .service(rethink_step);
}


#[post("/pair-programmer/generate-steps")]
pub async fn pair_programmer_generate_steps(
    data: web::Json<GenerateStepsRequest>,
    client: web::Data<Client>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {

    let user_id = data.user_id.clone().unwrap_or_else(|| "user_id".to_string());

    let session_id = match check_session(data.session_id.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(
                actix_web::error::ErrorInternalServerError(
                    json!({
                "error": e.to_string()
            })
                )
            );
        }
    };

    if data.task.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Task is required"
        })));
    }
    let pair_programmer_id = Uuid::new_v4().to_string();

    
    if let Some(files) = &data.files {
        for file_path in files {

            match index_code(&user_id, &pair_programmer_id, file_path).await {
                Ok(_) => {
                }
                Err(e) => {
                    return Err(
                        actix_web::error::ErrorInternalServerError(json!({ "error": e.to_string() }))
                    );
                }
            }
        }

    }

    let embeddings_result = generate_text_embedding(&data.task).await;
    let query_embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => {
            return Ok(
                HttpResponse::BadRequest().json(
                    serde_json::json!({
            "message": "No Matching result found", 
            "result": []
        })
                )
            );
        }
    };

    let chunk_ids = search_index(&pair_programmer_id, query_embeddings.clone(), 20);

    //  let file_path, chunk_type, content, pair_programmer_id;
    let entries = DB_INSTANCE.get_row_ids(chunk_ids).unwrap();
    info!("All the matching entries {:?}", entries);
    let formatted_entries: String = entries
    .iter()
    .map(|(file_path, _, content, _)| format!("{}\n{}", file_path, content))
    .collect::<Vec<String>>()
    .join("\n\n");

    info!("Formatted entries:\n{}", formatted_entries);
    let user_prompt_with_context = format!(
        "{}\nCONTEXT_CODE\n{}",
        data.task.clone(),
        formatted_entries
    );


    // This variable will accumulate the entire content of the stream
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    let agent = AgentEnum::new("planner", user_prompt_with_context)?;


    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Start streaming and sending data to the client
    let response = stream_to_client(
        &client,
        agent,
        pair_programmer_id.clone(),
        accumulated_content_clone,
        tx,
    ).await?;

    // Clone the necessary values to move them into the async task
    let task_clone = data.task.clone();
    let user_id_clone = user_id.clone();
    let session_id_clone = session_id.clone();
    // Spawn a separate task to handle the stream completion
    tokio::spawn(async move {
        handle_stream_completion_planner(rx, accumulated_content, &task_clone, &user_id_clone, &session_id_clone, pair_programmer_id).await;
        
    });

    Ok(response)

}


// The correct handler for GET steps
#[get("/pair-programmer/steps/{pair_programmer_id}")]
async fn get_steps(path: web::Path<String>) -> Result<HttpResponse, Error> {
    // Use into_inner to get the inner String from the Path extractors
    let pair_programmer_id = path.into_inner();

    // Fetch the steps for the provided pair_programmer_id
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);
    let response = json!({
        "steps": steps
    });
    // Return the result as JSON
    Ok(HttpResponse::Ok().json(response))
    // Return the result as JSON
}


#[post("/pair-programmer/steps/execute")]
pub async fn execute_step(payload: web::Payload, client: web::Data<Client>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let data = web::Json::<ExecuteStepRequest>::from_request(&req, &mut payload.into_inner()).await;
    let valid_data = match data {
        Ok(valid_data) => {
            // Check if fields are empty and return early if any field is missing
            if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
                let error_response = ErrorResponse {
                    error: "Missing required fields: pair_programmer_id or step_number".to_string(),
                };
                return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
            }

            valid_data.into_inner() // Proceed if validation passes
        }
        Err(err) => {
            // Handle invalid JSON error
            let error_response = ErrorResponse {
                error: format!("Invalid JSON payload: {}", err),
            };
            return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
        }
    };


    let pair_programmer_id = valid_data.pair_programmer_id.clone();
    let step_number = &valid_data.step_number;
    let step_data = data_validation(&pair_programmer_id, step_number).unwrap();


    let embeddings_result = generate_text_embedding(&step_data.task_heading).await;
    let query_embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => {
            return Ok(
                HttpResponse::BadRequest().json(
                    serde_json::json!({
            "message": "No Matching result found", 
            "result": []
        })
                )
            );
        }
    };

    let chunk_ids = search_index(&pair_programmer_id, query_embeddings.clone(), 20);

    //  let file_path, chunk_type, content, session_id;
    let entries = DB_INSTANCE.get_row_ids(chunk_ids).unwrap();
    info!("All the matching entries {:?}", entries);
    let formatted_entries: String = entries
        .iter()
        .map(|(file_path, _, content, _)| format!("{}\n{}", file_path, content))
        .collect::<Vec<String>>()
        .join("\n\n");

    info!("Formatted entries:\n{}", formatted_entries);
    let step_context = format!(
        "{}\nCONTEXT_CODE\n{}",
        step_data.task_heading.clone(),
        formatted_entries
    );

    
    let task_with_context = prompt_with_context(&step_data.all_steps, 
                                                    &step_data.steps_executed_so_far, 
                                                    &step_data.task_heading, 
                                                    &step_context, "");
    // Match the function call and return the appropriate agent
    let agent = AgentEnum::new("llm", task_with_context)?;

    // This variable will accumulate the entire content of the stream
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Start streaming and sending data to the client
    let response = stream_to_client(
        &client, 
        agent,
        pair_programmer_id.clone(),
        accumulated_content_clone,
        tx,
    ).await?;

    // Spawn a separate task to handle the stream completion
    tokio::spawn(async move {
        handle_stream_completion_execute(rx, accumulated_content, pair_programmer_id, step_data.step_number).await;
    });

    Ok(response)

}

#[post("/pair-programmer/steps/chat_summary")]
pub async fn chat_summary(payload: web::Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
    
    let data: Result<web::Json<SummarizeChatRequest>, Error> = web::Json::<SummarizeChatRequest>::from_request(&req, &mut payload.into_inner()).await;
    let valid_data = match data {
        Ok(valid_data) => {
            // Check if fields are empty and return early if any field is missing
            if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
                let error_response = ErrorResponse {
                    error: "Missing required fields: pair_programmer_id or step_number".to_string(),
                };
                return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
            }

            valid_data.into_inner() // Proceed if validation passes
        }
        Err(err) => {
            // Handle invalid JSON error
            let error_response = ErrorResponse {
                error: format!("Invalid JSON payload: {}", err),
            };
            return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
        }
    };
    
    let pair_programmer_id = valid_data.pair_programmer_id.clone();
    let step_number = parse_step_number(&valid_data.step_number)?;
    info!("step_number={}", step_number);

    let step_chat = match DB_INSTANCE.step_chat_string(&pair_programmer_id, &step_number.to_string()){
        Ok(chat) => chat,
        Err(err) => {
            let error_response = ErrorResponse{
                error: format!("Failed to retrieve chat: {}", err),
            };
            return Ok(HttpResponse::InternalServerError().json(error_response));
        }
    };

    let result = get_attention_scores(&step_chat).await;
    let tokens = match result {
        Ok(tokens) => tokens,
        Err(e) => {
            let error_response = ErrorResponse {
                            error: format!("Failed to summarize chat: {}", e),
                        };
                        return Ok(HttpResponse::InternalServerError().json(error_response)); // Handle;
        }
    };

    let embeddings_result = generate_text_embedding(&step_chat).await;
    match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(e) =>  {
            let error_response = ErrorResponse {
                            error: format!("Failed to summarize chat: {}", e),
                        };
                        return Ok(HttpResponse::InternalServerError().json(error_response)); // Handle;
        },
    };

    let compressed_prompt = tokens.join(" ");
    debug!("Compressed Prompt {:?}", compressed_prompt);

    let summary = summarize_text(&step_chat).await.unwrap();
    Ok(HttpResponse::Ok().json(json!({ "summary": summary })))
}

#[post("/pair-programmer/steps/chat")]
pub async fn chat_step(payload: web::Payload, client: web::Data<Client>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let data: Result<web::Json<ChatStepRequest>, Error> = web::Json::<ChatStepRequest>::from_request(&req, &mut payload.into_inner()).await;
    let valid_data = match data {
        Ok(valid_data) => {
            // Check if fields are empty and return early if any field is missing
            if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
                let error_response = ErrorResponse {
                    error: "Missing required fields: pair_programmer_id or step_number".to_string(),
                };
                return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
            }

            valid_data.into_inner() // Proceed if validation passes
        }
        Err(err) => {
            // Handle invalid JSON error
            let error_response = ErrorResponse {
                error: format!("Invalid JSON payload: {}", err),
            };
            return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
        }
    };


    let pair_programmer_id = valid_data.pair_programmer_id.clone();
    let step_number = &valid_data.step_number;
    let prompt = valid_data.prompt.clone();

    let step_data = data_validation(&pair_programmer_id, step_number).unwrap();

    
    let task_with_context=   prompt_with_context_for_chat(
                    &step_data.all_steps, 
                    &step_data.steps_executed_so_far, 
                    &step_data.task_heading, 
                    &prompt, "");
    // Match the function call and return the appropriate agent
    let agent = AgentEnum::new("chat", task_with_context)?;

    // This variable will accumulate the entire content of the stream

    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Start streaming and sending data to the client
    let response = stream_to_client(
        &client,
        agent,
        pair_programmer_id.clone(),
        accumulated_content_clone,
        tx,
    ).await?;

    // Spawn a separate task to handle the stream completion
    tokio::spawn(async move {
        handle_stream_completion_chat(rx, accumulated_content, pair_programmer_id, &prompt, step_data.step_number).await;
    });

    Ok(response)
}




#[post("/pair-programmer/steps/rethink")]
pub async fn rethink_step(payload: web::Payload, client: web::Data<Client>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let data: Result<web::Json<RethinkRequest>, Error> = web::Json::<RethinkRequest>::from_request(&req, &mut payload.into_inner()).await;
    let valid_data = match data {
        Ok(valid_data) => {
            // Check if fields are empty and return early if any field is missing
            if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
                let error_response = ErrorResponse {
                    error: "Missing required fields: pair_programmer_id or step_number".to_string(),
                };
                return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
            }

            valid_data.into_inner() // Proceed if validation passes
        }
        Err(err) => {
            // Handle invalid JSON error
            let error_response = ErrorResponse {
                error: format!("Invalid JSON payload: {}", err),
            };
            return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
        }
    };

    let pair_programmer_id = valid_data.pair_programmer_id.clone();
    let step_number = &valid_data.step_number;

    let step_data = data_validation(&pair_programmer_id, step_number).unwrap();

    

    let task_with_context=   rethink_prompt_with_context(
                                &step_data.all_steps, 
                                &step_data.steps_executed_so_far, 
                                &step_data.task_heading, 
                                &step_data.step_chat);
    // Match the function call and return the appropriate agent
    let agent = AgentEnum::new("rethinker", task_with_context)?;

    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Start streaming and sending data to the client
    let response = stream_to_client(
        &client,
        agent,
        pair_programmer_id.clone(),
        accumulated_content_clone,
        tx,
    ).await?;

    // Spawn a separate task to handle the stream completion
    tokio::spawn(async move {
        handle_stream_completion_rethinker(rx, accumulated_content, pair_programmer_id, step_data.step_number).await;
    });

    Ok(response)

}

async fn stream_to_client(
    client: &Client,
    agent: AgentEnum,
    pair_programmer_id: String,
    accumulated_content_clone: Arc<Mutex<String>>,
    tx: tokio::sync::oneshot::Sender<()>
) -> Result<HttpResponse, Error> {
    let stream_result = agent.execute(&client).await;
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
                  // Ensure `chunk` is a reference to `[u8]` or `Bytes`
                Ok(chunk) => {
                    if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                        // Accumulate the content in memory
                        {
                            let mut accumulated = accumulated_content_clone.lock().unwrap();
                            accumulated.push_str(chunk_str);
                        }

                        // Yield each chunk to the stream
                        yield Ok::<web::Bytes, Error>(web::Bytes::from(chunk_str.to_owned()));
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
        .append_header(("X-Pair-Programmer-id", pair_programmer_id.clone())) // Add the header here
        .streaming(response_stream);

    Ok(response)
}

async fn handle_stream_completion_rethinker(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    _pair_programmer_id: String,
    _step_number: usize
) {
    // Wait until the channel receives the completion signal
    let _ = rx.await;

    // Unwrap the accumulated content after streaming is done
    let accumulated_content_final = Arc::try_unwrap(accumulated_content)
        .unwrap_or_else(|_| Mutex::new(String::new()))
        .into_inner()
        .unwrap();

    // Print the accumulated content after streaming is completed
    println!("Final accumulated content: {}", accumulated_content_final);

    // Update step chat in the database after the stream completes
    // if let Err(err) = DB_INSTANCE.update_step_chat(&pair_programmer_id.clone(), &step_number.to_string(), &accumulated_content_final) {
    //     error!("Error updating chats array pair_programmer_id {} and step {}: {:?}", pair_programmer_id, step_number, err);
    // } else {
    //     debug!("DB Update successful for chat array pair_programmer_id {} and step {}", pair_programmer_id, step_number);
    // }
}

async fn handle_stream_completion_chat(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    pair_programmer_id: String,
    prompt: &str,
    step_number: usize
) {
    // Wait until the channel receives the completion signal
    let _ = rx.await;

    // Unwrap the accumulated content after streaming is done
    let accumulated_content_final = Arc::try_unwrap(accumulated_content)
        .unwrap_or_else(|_| Mutex::new(String::new()))
        .into_inner()
        .unwrap();

    // Print the accumulated content after streaming is completed
    println!("Final accumulated content: {}", accumulated_content_final);

    let db_response = DB_INSTANCE.update_step_chat(&pair_programmer_id.clone(), &step_number.to_string(), &prompt, &accumulated_content_final);
    match  db_response {
        Ok(_) => {debug!("DB Update successful for chat array pair_programmer_id {} and  step {}", pair_programmer_id, step_number)},
        Err(err) => {error!("Error updating chats array pair_programmer_id {} and  step {}: {:?}",  pair_programmer_id, step_number, err);}
    }
}

async fn handle_stream_completion_execute(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    pair_programmer_id: String,
    step_number: usize
) {
    // Wait until the channel receives the completion signal
    let _ = rx.await;

    // Unwrap the accumulated content after streaming is done
    let accumulated_content_final = Arc::try_unwrap(accumulated_content)
        .unwrap_or_else(|_| Mutex::new(String::new()))
        .into_inner()
        .unwrap();

    // Print the accumulated content after streaming is completed
    println!("Final accumulated content: {}", accumulated_content_final);

    let db_response = DB_INSTANCE.update_step_execution(&pair_programmer_id.clone(), &step_number.to_string(), &accumulated_content_final);
    match  db_response {
        Ok(_) => {debug!("DB Update successful for executing pair_programmer_id {} and  step {}", pair_programmer_id, step_number)},
        Err(err) => {error!("Error updating executing pair_programmer_id {} and  step {}: {:?}",  pair_programmer_id, step_number, err);}
    }
}

async fn handle_stream_completion_planner(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    task: &str,
    user_id: &str,
    session_id: &str,
    pair_programmer_id: String
    ) {
    // Wait until the channel receives the completion signal
    let _ = rx.await;

    // Unwrap the accumulated content after streaming is done
    let accumulated_content_final = Arc::try_unwrap(accumulated_content)
        .unwrap_or_else(|_| Mutex::new(String::new()))
        .into_inner()
        .unwrap();

    let json_data = accumulated_content_final
        .replace("```json", "") // Remove the opening marker
        .replace("```", ""); 
    
    info!("Json data {}", json_data);
    // Attempt to parse and handle errors gracefully
    let steps: Vec<crate::pair_programmer::types::Step> = match parse_steps(&json_data) {
        Ok(parsed_steps) => parsed_steps,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            return; // Exit early if parsing fails
        }
    };


    // Print the accumulated content after streaming is completed
    println!("Steps = {:?}", steps);

    let db_response= DB_INSTANCE.store_new_pair_programming_session(user_id, session_id, &pair_programmer_id, task, &steps);
    match  db_response {
        Ok(_) => {debug!("DB Update successful for planning the task at id {} and number of steps step {}", pair_programmer_id, steps.len())},
        Err(err) => {error!("Error inserting for planning the task at id {} and number of steps step {} with error {:?}",  pair_programmer_id, steps.len(), err);}
    }
}