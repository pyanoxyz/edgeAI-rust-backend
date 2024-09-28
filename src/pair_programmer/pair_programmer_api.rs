use actix_web::{post, web, HttpRequest, HttpResponse, Error};
use crate::chats::chat_struct::{RefactorPrompt, handle_request};
use serde::{Deserialize, Serialize};
use crate::request_type::RequestType;
use crate::pair_programmer::agent_planner::PlannerAgent;
use crate::pair_programmer::agent::Agent;
use serde_json::json;
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateStepsRequest {
    pub task: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,

}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(pair_programmer_generate_steps); // Register the correct route handler
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

    let planner_agent = PlannerAgent::new(data.task.clone(), data.task.clone());
    planner_agent.execute()
    .await
    .map_err(|e| {
        actix_web::error::ErrorInternalServerError(json!({
            "error": format!("Local LLM response error: {}", e)
        }))
    })
    
}
