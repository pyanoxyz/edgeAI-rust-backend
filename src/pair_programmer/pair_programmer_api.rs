use actix_web::{post, web, get, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
use crate::request_type::RequestType;
use crate::pair_programmer::agent_planner::PlannerAgent;
use crate::pair_programmer::agent::Agent;
use uuid::Uuid;
use crate::database::db_config::DB_INSTANCE;

use serde_json::json;
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateStepsRequest {
    pub task: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,

}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(pair_programmer_generate_steps)
    .service(get_steps); // Register the correct route handler
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
async fn get_steps(path: web::Path<String>) -> HttpResponse {
    // Use into_inner to get the inner String from the Path extractor
    let pair_programmer_id = path.into_inner();

    // Fetch the steps for the provided pair_programmer_id
    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);

    // Return the result as JSON
    HttpResponse::Ok().json(steps)
}
