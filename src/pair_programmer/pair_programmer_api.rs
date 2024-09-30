use actix_web::{post, web, get, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
use crate::request_type::RequestType;
use crate::pair_programmer::{agent_planner::PlannerAgent, agent_enum::AgentEnum};
use crate::pair_programmer::agent::Agent;
use uuid::Uuid;
use crate::database::db_config::DB_INSTANCE;
use log::info;
use serde_json::Value; // For handling JSON data
use std::cmp::max;
use actix_web::error::JsonPayloadError;
use actix_web::FromRequest; // Import this trait to use `from_request`

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




pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(pair_programmer_generate_steps)
    .service(get_steps)
    .service(execute_step); // Register the correct route handler
}

#[post("/pair-programmer/generate-steps")]
pub async fn pair_programmer_generate_steps(data: web::Json<GenerateStepsRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {

    let user_id = data.user_id.clone().unwrap_or_else(|| "user_id".to_string());

    let session_id = match &data.session_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Session ID is required"
            })));
        }
    };

    if session_id.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Session ID is required"
        })));
    }
    if data.task.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Task is required"
        })));
    }

    let pair_programmer_id = Uuid::new_v4().to_string();

    let planner_agent = PlannerAgent::new(data.task.clone(), data.task.clone());
    planner_agent.execute(&user_id, &session_id, &pair_programmer_id)
    .await
    .map_err(|e| {
        actix_web::error::ErrorInternalServerError(json!({
            "error": format!("Local LLM response error: {}", e)
        }))
    })
    
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

    let pair_programmer_id = &valid_data.pair_programmer_id;

    info!("PairProgrammerId={}", pair_programmer_id);

    info!("PairProgrammerId={}", pair_programmer_id);
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

    agent.execute("", "", &pair_programmer_id)
    .await
    .map_err(|e| {
        actix_web::error::ErrorInternalServerError(json!({
            "error": format!("Local LLM response error: {}", e)
        }))
    })

    // Ok(HttpResponse::Ok().json(json!({
    //     "message": format!("Step {} can be executed", step_number)
    // })))

}


// Helper function to parse the step_number from a string to usize
fn parse_step_number(step_number_str: &str) -> Result<usize, Error> {
    step_number_str
        .parse::<usize>()
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid step number: unable to convert to a valid number"))
}

// Helper function to validate whether the steps can be executed
fn validate_steps(step_number: usize, steps: &Vec<serde_json::Value>) -> Result<(), Error> {
    if step_number > steps.len() {
        return Err(actix_web::error::ErrorBadRequest(
            format!("Step number {} is out of bounds, there are only {} steps", step_number, steps.len()),
        ));
    }

    for (index, step) in steps.into_iter().enumerate() {
        let actual_index = index + 1; // Start enumeration from 1

        // Access step data as an object
        let step_data = step.as_object().ok_or_else(|| {
            actix_web::error::ErrorInternalServerError("Invalid step data format")
        })?;

        // Check if the current step is the one we want to execute
        if actual_index == step_number {
            let executed = step_data.get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // If the step is already executed, return an error
            if executed {
                return Err(actix_web::error::ErrorBadRequest(
                    format!("Step {} has already been executed", step_number),
                ));
            }
        }

        // Ensure that all previous steps are executed
        if actual_index < step_number {
            let previous_executed = step_data.get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !previous_executed {
                return Err(actix_web::error::ErrorBadRequest(
                    format!("Previous step {} has not been executed", actual_index),
                ));
            }
        }
    }
    Ok(())
}

fn format_steps(steps: &[Value], step_number: usize) -> (String, String, String) {
    // Format all steps
    let all_steps = steps.iter()
        .enumerate()
        .map(|(index, step)| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            format!("Step: {}. {}", index + 1, heading)
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Format steps executed so far
    let steps_executed_so_far = steps.iter()
        .enumerate()
        .filter(|(_, step)| step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|(index, step)| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            format!("Step: {}. {}", index + 1, heading)
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Format steps executed with response (limit to last 3 before current step_number)
    let steps_executed_with_response = steps.iter()
        .skip(max(0, step_number.saturating_sub(3)))  // Start from max(0, step_number-3)
        .take(step_number)  // Take up to current step_number
        .filter(|step| step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|step| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            let response = step.get("response").and_then(|v| v.as_str()).unwrap_or("No Response");
            format!("Step: {}\n response: {}\n", heading, response)
        })
        .collect::<Vec<String>>()
        .join("\n");

    (all_steps, steps_executed_so_far, steps_executed_with_response)
}

fn prompt_with_context(
    all_steps: &str, 
    steps_executed: &str, 
    current_step: &str, 
    additional_context_from_codebase: &str, 
    recent_discussion: &str
) -> String {
    format!(
        r#"
        all_steps: {all_steps}
        steps_executed_so_far: {steps_executed}
        current_step: {current_step}
        overall_context: {additional_context_from_codebase}
        recent_discussion: {recent_discussion}
        Please implement the current step based on this context. Ensure your response follows the specified output format in the system prompt.
        "#,
        all_steps = all_steps,
        steps_executed = steps_executed,
        current_step = current_step,
        additional_context_from_codebase = additional_context_from_codebase,
        recent_discussion = recent_discussion
    )
}