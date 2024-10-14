use actix_web::{ post, web, HttpRequest, HttpResponse, Error };
use super::state::InfillModelState;
use serde::{ Deserialize, Serialize };
use std::sync::Arc ;
use reqwest::Client;
use super::stream_utils::stream_infill_request;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct InfillRequest {
    pub code_before: String,
    pub code_after: String,
    pub infill_id: String,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(infill); // Register the correct route handler
}

#[post("/chat/infill")]
pub async fn infill(
    data: web::Json<InfillRequest>,
    infill_model_state: web::Data<Arc<InfillModelState>>,
    client: web::Data<Client>,
    _req: HttpRequest
) -> Result<HttpResponse, Error> {
    let infill_id = &data.infill_id;

    let model_process_guard = infill_model_state.infill_model_process.lock().await;

    let model_running = model_process_guard.is_some();
    if !model_running {
        return Ok(
            HttpResponse::InternalServerError().json(json!({"error": "Infill model not running"}))
        );
    }

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

    let (tx, _rx) = tokio::sync::oneshot::channel::<()>();

    // Adding context for the infill will increase generation time

    let response = stream_infill_request(&client, &infill_id, infill_req_body, tx).await?;

    Ok(response)
}
