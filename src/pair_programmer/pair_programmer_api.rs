use uuid::Uuid;
use serde_json::json;
use async_stream::stream;
use actix_web::FromRequest;
// use futures_util::{Stream, StreamExt}; // Import this trait for accessing `.next()`
use std::sync::{Arc, Mutex};
use log::{info, debug, error};
use serde::{Deserialize, Serialize};
use crate::context::store_text_context::index_code;
use crate::similarity_index::index::search_index;
use crate::pair_programmer::agent::Agent;
use crate::database::db_config::DB_INSTANCE;
use crate::pair_programmer::agent_enum::AgentEnum;
use crate::embeddings::text_embeddings::generate_text_embedding;
use actix_web::{post, web, get, HttpRequest, HttpResponse, Error};
use crate::pair_programmer::pair_programmer_utils::{parse_steps, parse_step_number, prompt_with_context};
use futures::StreamExt; // Ensure StreamExt is imported
use crate::session_manager::check_session;
use reqwest::Client;

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
    .service(test_json_parsing);
}

#[get("/pair-programmer/test_json_parsing")]
pub async fn test_json_parsing(
    _client: web::Data<Client>,

) -> Result<HttpResponse, Error> {
    let input = r#"
   ```yaml
```yaml
steps:
  - step_number: "1"
    heading: "Create a new directory for project files if it doesn't exist."
    action: "create_directory"
    details:
      directory: "./csv_parser_project"
  - step_number: "2"
    heading: "Navigate into the newly created project directory."
    action: "system_command"
    details:
      command: "cd ./csv_parser_project"
  - step_number: "3"
    heading: "Create a new Python virtual environment for this project if it doesn't exist."
    action: "create_directory" # Assuming we create venv as a subdirectory
    details:
      directory: "./venv"
  - step_number: "4"
    heading: "Activate the newly created Python virtual environment."
    action: "system_command"
    details:
      command: "source ./venv/bin/activate" # For Unix-based systems
  - step_step_number: "5"
    heading: "Install required packages (pandas, sqlalchemy) into the activated virtual environment using pip."
    action: "install_dependency"
    details:
      package_name: "pip install pandas sqlalchemy"
  - step_number: "6"
    heading: "Create a new Python script file named 'parse_csv.py' if it doesn't already exist in the project directory."
    action: "create_file"
    details:
      filename: "./csv_parser_project/parse_csv.py"
  - step_number: "7"
    heading: "Edit the newly created Python script to include necessary imports and CSV parsing logic using pandas library. Also, set up database connection with SQLAlchemy."
    action: "edit_file"
    details:
      filename: "./csv_parser_project/parse_csv.py"
  - step_step_number: "8"
    heading: "Write a function in the 'parse_csv.py' script to handle large CSV file parsing efficiently using pandas chunking feature. Ensure it handles memory usage effectively."
    action: "edit_file" # Continue editing parse_csv.py
    details:
      filename: "./csv_parser_project/parse_csv.py"
  - step_number: "9"
    heading: "Write a function in the 'parse_csv.py' script to store parsed data into an SQL database using SQLAlchemy ORM. Ensure it handles different data types and relationships."
    action: "edit_file" # Continue editing parse_csv.py
    details:
      filename: "./csv_parser_project/parse_csv.py"
  - step_number: "10"
    heading: "Test the 'parse_csv.py' script by running a test case with a sample CSV file. Ensure it correctly parses and stores data."
    action: "run_tests" # Assuming there's already a testing framework set up
    details:
      command: "./csv_parser_project/venv/bin/python -m unittest discover ./test"
  - step_number: "11"
    heading: "Document the steps taken to create, activate virtual environment and install dependencies."
    action: "edit_file" # Create or edit a README.md file
    details:
      filename: "./csv_parser_project/README.md"    
      "#;

    // Call the custom parser to parse steps
    let steps = parse_steps(input).unwrap_or_else(|err| {
        eprintln!("Error parsing steps: {:?}", err);
        vec![]
    });    
    
    println!("Printing steps now");
    for step in &steps {
        println!("{:?}", step);
    }

    Ok(HttpResponse::Ok().json(steps))

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

            match index_code(&user_id, &session_id, file_path).await {
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

    let chunk_ids = search_index(&session_id, query_embeddings.clone(), 20);

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
    //fetching the step details that is to be executed
    let step_number_usize = parse_step_number(step_number).map_err(|err| {
        actix_web::error::ErrorBadRequest(format!("Invalid step number: {}", err))
    })?;

    //fetching all steps for the pai_programmer_id
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);
    let all_steps = steps.iter()
            .enumerate()
            .map(|(index, step)| {
                let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
                format!("Step: {}. {}", index + 1, heading)
            })
            .collect::<Vec<String>>()
            .join("\n");

    // Format steps executed with response (output all steps before the current step_number)
    let steps_executed_with_response = steps.iter()
        .take(step_number_usize)  // Take all steps up to the current step_number
        .filter(|step| {
            step.get("tool").and_then(|v| v.as_str()) == Some("edit_file") &&
            step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false)
        })
        .map(|step| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            let response = step.get("response").and_then(|v| v.as_str()).unwrap_or("No Response");
            format!("Step: {}\n response: {}\n", heading, response)
        })
        .collect::<Vec<String>>()
        .join("\n");
    
    let step_result = DB_INSTANCE.fetch_single_step(&pair_programmer_id, step_number);

    let step = match step_result {
        Ok(step) => step,
        Err(e) => {
            error!("Error fetching step {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Error fetching step: {}", e)
            })));
        } 

    };
    
    if step.action != "edit_file"{
        let db_response = DB_INSTANCE.update_step_execution(&pair_programmer_id.clone(), &step_number.to_string(), "");
        return Ok(HttpResponse::Ok().json(json!({"message": "Marked as executed"})));
    }

    // let step_data = match data_validation(&pair_programmer_id, step_number) {
    //     Ok(step_data) => step_data, // Proceed if validation succeeds
    //     Err(err) => {
    //         // Handle validation failure
    //         let error_response = ErrorResponse {
    //             error: format!("Data validation failed: {}", err),
    //         };
    //         return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
    //     }
    // };

    let embeddings_result = generate_text_embedding(&step.heading).await;
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
        step.heading.clone(),
        formatted_entries
    );

    
    let task_with_context = prompt_with_context(&pair_programmer_id,
                                                    &all_steps, 
                                                    &steps_executed_with_response, 
                                                    &step.heading, 
                                                    &step_context);
    // Match the function call and return the appropriate agent

    info!("Task With COntext:\n{}", task_with_context);


    let agent = AgentEnum::new("generate-code", task_with_context)?;

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
        handle_stream_completion_execute(rx, accumulated_content, pair_programmer_id, step.step_number).await;
    });

    Ok(response)

}

// #[post("/pair-programmer/steps/chat_summary")]
// pub async fn chat_summary(payload: web::Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
    
//     let data: Result<web::Json<SummarizeChatRequest>, Error> = web::Json::<SummarizeChatRequest>::from_request(&req, &mut payload.into_inner()).await;
//     let valid_data = match data {
//         Ok(valid_data) => {
//             // Check if fields are empty and return early if any field is missing
//             if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
//                 let error_response = ErrorResponse {
//                     error: "Missing required fields: pair_programmer_id or step_number".to_string(),
//                 };
//                 return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
//             }

//             valid_data.into_inner() // Proceed if validation passes
//         }
//         Err(err) => {
//             // Handle invalid JSON error
//             let error_response = ErrorResponse {
//                 error: format!("Invalid JSON payload: {}", err),
//             };
//             return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
//         }
//     };
    
//     let pair_programmer_id = valid_data.pair_programmer_id.clone();
//     let step_number = parse_step_number(&valid_data.step_number)?;
//     info!("step_number={}", step_number);

//     let step_chat = match DB_INSTANCE.step_chat_string(&pair_programmer_id, &step_number.to_string()){
//         Ok(chat) => chat,
//         Err(err) => {
//             let error_response = ErrorResponse{
//                 error: format!("Failed to retrieve chat: {}", err),
//             };
//             return Ok(HttpResponse::InternalServerError().json(error_response));
//         }
//     };

//     let result = get_attention_scores(&step_chat).await;
//     let tokens = match result {
//         Ok(tokens) => tokens,
//         Err(e) => {
//             let error_response = ErrorResponse {
//                             error: format!("Failed to summarize chat: {}", e),
//                         };
//                         return Ok(HttpResponse::InternalServerError().json(error_response)); // Handle;
//         }
//     };

//     let embeddings_result = generate_text_embedding(&step_chat).await;
//     match embeddings_result {
//         Ok(embeddings) => embeddings,
//         Err(e) =>  {
//             let error_response = ErrorResponse {
//                             error: format!("Failed to summarize chat: {}", e),
//                         };
//                         return Ok(HttpResponse::InternalServerError().json(error_response)); // Handle;
//         },
//     };

//     let compressed_prompt = tokens.join(" ");
//     debug!("Compressed Prompt {:?}", compressed_prompt);

//     let summary = summarize_text(&step_chat).await.unwrap();
//     Ok(HttpResponse::Ok().json(json!({ "summary": summary })))
// }

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
    // let step_number = parse_step_number(step_number).map_err(|err| {
    //     actix_web::error::ErrorBadRequest(format!("Invalid step number: {}", err))
    // })?;


    //Prompt by the user to make changes to the code
    let prompt = valid_data.prompt.clone();

    let step_result = DB_INSTANCE.fetch_single_step(&pair_programmer_id, step_number);

    let step = match step_result {
        Ok(step) => step,
        Err(e) => {
            error!("Error fetching step {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Error fetching step: {}", e)
            })));
        } 

    };

    //Generate embeddings for the task
    let embeddings_result = generate_text_embedding(&step.heading).await;
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

    //Search index if there are any matching code snippets in the whole code repo - matching with task heading
    let chunk_ids = search_index(&pair_programmer_id, query_embeddings.clone(), 20);

    //  let file_path, chunk_type, content, session_id;
    // chunk_ids will give the u64 unique ids of the code chunks stored in the index, This
    //step will fetch the actual code snippets from the database
    let entries = DB_INSTANCE.get_row_ids(chunk_ids).unwrap();
    info!("All the matching entries {:?}", entries);
    let formatted_entries: String = entries
        .iter()
        .map(|(file_path, _, content, _)| format!("{}\n{}", file_path, content))
        .collect::<Vec<String>>()
        .join("\n\n");

    info!("Formatted entries:\n{}", formatted_entries);

    //fetching all steps for the pai_programmer_id
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);
    let all_steps = steps.iter()
            .enumerate()
            .map(|(index, step)| {
                let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
                format!("Step: {}. {}", index + 1, heading)
            })
            .collect::<Vec<String>>()
            .join("\n");

    let user_prompt_with_context = format!(
        "ALL_STEPS ={}\nTASK ={}\nRESPONSE = {}\nCONTEXT_CODE = {}\n USER_PROMPT={}",
        all_steps,
        step.heading.clone(),
        step.response,
        formatted_entries,
        prompt
    );
    // Match the function call and return the appropriate agent
    let agent;
    if step.response == ""{
        info!("The task hasnt been executed and hence the task heading will be updated by the LLM response");
        agent = AgentEnum::new("modifystep", user_prompt_with_context)?;
    }else{
        info!("The task has been executed and hence the code will be modified by the llm response");
        agent = AgentEnum::new("modifycode", user_prompt_with_context)?;
    }
    
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
        handle_stream_completion_chat(rx, accumulated_content, pair_programmer_id, &prompt, step.step_number, step.response).await;
    });

    Ok(response)
}




// #[post("/pair-programmer/steps/rethink")]
// pub async fn rethink_step(payload: web::Payload, client: web::Data<Client>, req: HttpRequest) -> Result<HttpResponse, Error> {
//     let data: Result<web::Json<RethinkRequest>, Error> = web::Json::<RethinkRequest>::from_request(&req, &mut payload.into_inner()).await;
//     let valid_data = match data {
//         Ok(valid_data) => {
//             // Check if fields are empty and return early if any field is missing
//             if valid_data.pair_programmer_id.trim().is_empty() || valid_data.step_number.trim().is_empty() {
//                 let error_response = ErrorResponse {
//                     error: "Missing required fields: pair_programmer_id or step_number".to_string(),
//                 };
//                 return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if validation fails
//             }

//             valid_data.into_inner() // Proceed if validation passes
//         }
//         Err(err) => {
//             // Handle invalid JSON error
//             let error_response = ErrorResponse {
//                 error: format!("Invalid JSON payload: {}", err),
//             };
//             return Ok(HttpResponse::BadRequest().json(error_response)); // Return early if JSON is invalid
//         }
//     };

//     let pair_programmer_id = valid_data.pair_programmer_id.clone();
//     let step_number = &valid_data.step_number;

//     let step_data = data_validation(&pair_programmer_id, step_number).unwrap();

    

//     let task_with_context=   rethink_prompt_with_context(
//                                 &step_data.all_steps, 
//                                 &step_data.steps_executed_response, 
//                                 &step_data.task_heading, 
//                                 &step_data.step_chat);
//     // Match the function call and return the appropriate agent
//     let agent = AgentEnum::new("rethinker", task_with_context)?;

//     let accumulated_content = Arc::new(Mutex::new(String::new()));
//     let accumulated_content_clone = Arc::clone(&accumulated_content);
//     let (tx, rx) = tokio::sync::oneshot::channel::<()>();

//     // Start streaming and sending data to the client
//     let response = stream_to_client(
//         &client,
//         agent,
//         pair_programmer_id.clone(),
//         accumulated_content_clone,
//         tx,
//     ).await?;

//     // Spawn a separate task to handle the stream completion
//     tokio::spawn(async move {
//         handle_stream_completion_rethinker(rx, accumulated_content, pair_programmer_id, step_data.step_number).await;
//     });

//     Ok(response)

// }

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

// async fn handle_stream_completion_rethinker(
//     rx: tokio::sync::oneshot::Receiver<()>,
//     accumulated_content: Arc<Mutex<String>>,
//     _pair_programmer_id: String,
//     _step_number: usize
// ) {
//     // Wait until the channel receives the completion signal
//     let _ = rx.await;

//     // Unwrap the accumulated content after streaming is done
//     let accumulated_content_final = Arc::try_unwrap(accumulated_content)
//         .unwrap_or_else(|_| Mutex::new(String::new()))
//         .into_inner()
//         .unwrap();

//     // Print the accumulated content after streaming is completed
//     println!("Final accumulated content: {}", accumulated_content_final);

//     // Update step chat in the database after the stream completes
//     // if let Err(err) = DB_INSTANCE.update_step_chat(&pair_programmer_id.clone(), &step_number.to_string(), &accumulated_content_final) {
//     //     error!("Error updating chats array pair_programmer_id {} and step {}: {:?}", pair_programmer_id, step_number, err);
//     // } else {
//     //     debug!("DB Update successful for chat array pair_programmer_id {} and step {}", pair_programmer_id, step_number);
//     // }
// }



async fn handle_stream_completion_execute(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    pair_programmer_id: String,
    step_number: String
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
    let steps: Vec<crate::pair_programmer::types::PairProgrammerStepRaw> = match parse_steps(&json_data) {
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


async fn handle_stream_completion_chat(
    rx: tokio::sync::oneshot::Receiver<()>,
    accumulated_content: Arc<Mutex<String>>,
    pair_programmer_id: String,
    prompt: &str,
    step_number: String, 
    response: String
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
    let db_response;
    if response == ""{
        info!("The task hasnt been executed and hence the task heading will be updated by the LLM response");
        //if response is not empty that means the modifycode agent has been executed and it updates the repsonse
        db_response = DB_INSTANCE.update_step_heading(&pair_programmer_id.clone(), &step_number.to_string(), &prompt, &accumulated_content_final);

    }else{
        //if response is empty that means the modifystep agent has been executed and it updates the task heading
        db_response = DB_INSTANCE.update_step_response(&pair_programmer_id.clone(), &step_number.to_string(), &prompt, &accumulated_content_final);
    }

    // let db_response = DB_INSTANCE.update_step_chat(&pair_programmer_id.clone(), &step_number.to_string(), &prompt, &accumulated_content_final);
    match  db_response {
        Ok(_) => {debug!("DB Update successful for chat array pair_programmer_id {} and  step {}", pair_programmer_id, step_number)},
        Err(err) => {error!("Error updating chats array pair_programmer_id {} and  step {}: {:?}",  pair_programmer_id, step_number, err);}
    }
}