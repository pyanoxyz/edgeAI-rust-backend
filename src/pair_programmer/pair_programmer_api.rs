use actix_web::{post, web, get, HttpRequest, HttpResponse, Error};
use serde::{Deserialize, Serialize};
use crate::pair_programmer::{agent_planner::PlannerAgent, agent_enum::AgentEnum};
use crate::pair_programmer::agent::Agent;
use uuid::Uuid;
use crate::database::db_config::DB_INSTANCE;
use log::{info, debug, error};
use actix_web::FromRequest;

// Import this trait to use `from_request`
use std::sync::{Arc, Mutex};    
use futures_util::StreamExt; // Import this trait for accessing `.next()`
use async_stream::stream;
use crate::pair_programmer::pair_programmer_utils::{parse_steps, validate_steps, parse_step_number, format_steps, prompt_with_context, prompt_with_context_for_chat };
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateStepsRequest {
    pub task: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,

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
    .service(chat_step);
}


/// Handle the end of the stream by processing accumulated content
/// This user_prompt is the original prompt that user has gave us
/// not the prompt with context because that has already been passed in llm_request
async fn parse_steps_and_store(
    input: &str,
    user_prompt: &str,
    user_id: &str,
    session_id: &str,
    pair_programmer_id: &str
) {
    let steps = parse_steps(input);
    for step in &steps {
        println!("{:?}", step);
    }
    DB_INSTANCE.store_new_pair_programming_session(user_id, session_id, pair_programmer_id, user_prompt, &steps); 
}

#[post("/pair-programmer/generate-steps")]
pub async fn pair_programmer_generate_steps(
    data: web::Json<GenerateStepsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {

    let user_id = data.user_id.clone().unwrap_or_else(|| "user_id".to_string());

    let session_id = match &data.session_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Session ID is required"
            })));
        }
    };

    if data.task.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Task is required"
        })));
    }

    // This variable will accumulate the entire content of the stream
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    let pair_programmer_id = Uuid::new_v4().to_string();

    let planner_agent = PlannerAgent::new(data.task.clone(), data.task.clone());

    let stream_result = planner_agent.execute().await;
    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Local LLM response error: {}", e)
            })));
        }
    };

    // Create a channel to wait for the stream completion
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // Stream chunks to the client in real time and accumulate
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
        .append_header(("pair-programmer-id", pair_programmer_id.clone())) // Add the header here
        .streaming(response_stream);

    // Wait for the streaming to complete before unwrapping the accumulated content
    tokio::spawn(async move {
        // Wait until the channel receives the completion signal
        let _ = rx.await;

        // Unwrap the accumulated content after streaming is done
        let accumulated_content_final = Arc::try_unwrap(accumulated_content)
            .unwrap_or_else(|_| Mutex::new(String::new()))
            .into_inner()
            .unwrap();

        // Print the accumulated content after streaming is completed
        println!("Final accumulated content: {}", accumulated_content_final);
        parse_steps_and_store(
            &accumulated_content_final,
            &data.task.clone(),
            &user_id,
            &session_id,
            &pair_programmer_id
        ).await;
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

    // Return the result as JSON
    Ok(HttpResponse::Ok().json(steps))
}


#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}


#[post("/pair-programmer/steps/execute")]
pub async fn execute_step(payload: web::Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
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


    // Parse the step_number
    let step_number = parse_step_number(&valid_data.step_number)?;
    info!("step_number={}", step_number);

    // Fetch the steps from the database
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);
    // Check if the steps can be executed
    match validate_steps(step_number, &steps){
        Ok(_) => {},
        Err(error) => {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{}", error),
            })));        
        }
    }
    let step = &steps[step_number];

    //TODO: Last step execution shall also be given in context and also the chat
    let (all_steps, steps_executed_so_far, steps_executed_with_response) = format_steps(&steps, step_number);

    let function_call = step.get("tool")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest(format!("Invalid step: 'tool' field is missing or not a string {}", step_number))
        })
        .unwrap();

    let task_heading = step.get("heading")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest(format!("Invalid step: 'heading' field is missing or not a string {}", step_number))
        })
        .unwrap();

    let task_with_context=   prompt_with_context(&all_steps, &steps_executed_so_far, task_heading, "", "");
    // Match the function call and return the appropriate agent
    let agent = AgentEnum::new(function_call, task_heading.to_string(), task_with_context)?;

    // This variable will accumulate the entire content of the stream
      let accumulated_content = Arc::new(Mutex::new(String::new()));
      let accumulated_content_clone = Arc::clone(&accumulated_content);
  
    
      let stream_result = agent.execute().await;
      let mut stream = match stream_result {
          Ok(s) => s,
          Err(e) => {
              return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                  "error": format!("Local LLM response error: {}", e)
              })));
          }
      };
  
      // Create a channel to wait for the stream completion
      let (tx, rx) = tokio::sync::oneshot::channel::<()>();

      // Stream chunks to the client in real time and accumulate
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
          .append_header(("pair-programmer-id", pair_programmer_id.clone())) // Add the header here
          .streaming(response_stream);
  
      // Wait for the streaming to complete before unwrapping the accumulated content
      tokio::spawn(async move {
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

      });

    Ok(response)

}


#[post("/pair-programmer/steps/chat")]
pub async fn chat_step(payload: web::Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
    let data = web::Json::<ChatStepRequest>::from_request(&req, &mut payload.into_inner()).await;
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
    let prompt = valid_data.prompt.clone();


    // Parse the step_number
    let step_number = parse_step_number(&valid_data.step_number)?;
    info!("step_number={}", step_number);
    let true_step_number = step_number -1;

    // Fetch the steps from the database
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);
    // Check if the steps can be executed
    match validate_steps(step_number, &steps){
        Ok(_) => {},
        Err(error) => {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": format!("{}", error),
            })));        
        }
    }
    let step = &steps[true_step_number];

    //TODO: Last step execution shall also be given in context and also the chat
    let (all_steps, steps_executed_so_far, steps_executed_with_response) = format_steps(&steps, step_number);

    // let function_call = step.get("tool")
    //     .and_then(|v| v.as_str())
    //     .ok_or_else(|| {
    //         actix_web::error::ErrorBadRequest(format!("Invalid step: 'tool' field is missing or not a string {}", step_number))
    //     })
    //     .unwrap();
    
    //here we also use step number because the indexing in the database starts with 1 not 0.
    let step_chats = DB_INSTANCE.get_step_chat(&pair_programmer_id, &step_number.to_string());
    info!("Chat history {:?}", step_chats);
    
    let task_heading = step.get("heading")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest(format!("Invalid step: 'heading' field is missing or not a string {}", true_step_number))
        })
        .unwrap();

    let task_with_context=   prompt_with_context_for_chat(&all_steps, &steps_executed_so_far, task_heading, &prompt, "");
    // Match the function call and return the appropriate agent
    let agent = AgentEnum::new("chat", task_heading.to_string(), task_with_context)?;

    // This variable will accumulate the entire content of the stream
    let accumulated_content = Arc::new(Mutex::new(String::new()));
    let accumulated_content_clone = Arc::clone(&accumulated_content);

    
    let stream_result = agent.execute().await;
    let mut stream = match stream_result {
          Ok(s) => s,
          Err(e) => {
              return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                  "error": format!("Local LLM response error: {}", e)
              })));
          }
      };
  
      // Create a channel to wait for the stream completion, the receiver will wait till the end of the stream
      let (tx, rx) = tokio::sync::oneshot::channel::<()>();

      // Stream chunks to the client in real time and accumulate
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
          .append_header(("pair-programmer-id", pair_programmer_id.clone())) // Add the header here
          .streaming(response_stream);
  
      // Wait for the streaming to complete before unwrapping the accumulated content
      tokio::spawn(async move {
          // Wait until the channel receives the completion signal
          let _ = rx.await;
  
          // Unwrap the accumulated content after streaming is done
          let accumulated_content_final = Arc::try_unwrap(accumulated_content)
              .unwrap_or_else(|_| Mutex::new(String::new()))
              .into_inner()
              .unwrap();
  
          // Print the accumulated content after streaming is completed
          println!("Final accumulated content: {}", accumulated_content_final);
          //here step number should be step_number not tru_step_number because the steps are being stored rom an index 1 rather then zero.
          let db_response = DB_INSTANCE.update_step_chat(&pair_programmer_id.clone(), &step_number.to_string(), &prompt, &accumulated_content_final);
            match  db_response {
                Ok(_) => {debug!("DB Update successful for chat array pair_programmer_id {} and  step {}", pair_programmer_id, step_number)},
                Err(err) => {error!("Error updating chats array pair_programmer_id {} and  step {}: {:?}",  pair_programmer_id, step_number, err);}
            }

      });

    Ok(response)

}