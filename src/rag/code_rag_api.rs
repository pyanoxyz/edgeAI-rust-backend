use actix_web::{post, get, web, HttpRequest, HttpResponse, Responder, Error};
use serde::{Deserialize, Serialize};
use crate::authentication::authorization::is_request_allowed;
use log::{debug, info};
use crate::session_manager::check_session;
use serde_json::json;
use crate::rag::code_rag::index_code;
use crate::parser::parse_code::Chunk;
use crate::database::db_config::DB_INSTANCE;

#[derive(Debug, Serialize, Deserialize)]
pub struct RagRequest {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub files: Vec<String>, 
}


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(rag_request)
        .service(get_indexed_context); // Register the correct route handler
}

#[post("/rags/index/code")]
pub async fn rag_request(data: web::Json<RagRequest>, req: HttpRequest) -> Result<HttpResponse, Error> {
    
    info!("Session_id = {:?}", data.session_id.clone());
    info!("Files = {:?}",  data.files.clone());

    // Check session and extract user ID from the request
    let session_id = match check_session(data.session_id.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(actix_web::error::ErrorInternalServerError(json!({
                "error": e.to_string()
            })));
        }
    };

    let user_id: &str;
    let user_id_cloned;

    match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => {
            // Handle case when request is allowed and user is present
            user_id_cloned = user.user_id.clone();
            user_id = &user_id_cloned;
        }
        Ok(None) => {
            // Handle case when request is allowed but user is not found
            user_id = "user_id"; // Default user_id
        }
        Err(error_response) => {
            // Handle the error case by propagating the HttpResponse error
            return Err(actix_web::error::ErrorInternalServerError(json!({
                "error": "An error occurred during request validation"
            })));
        }
    }

    let mut all_indexed_chunks: Vec<Chunk> = Vec::new();
    // Iterate over the files and call `index_code` for each
    for file_path in &data.files {
        match index_code(user_id, &session_id, file_path).await {
            Ok(chunks) => {
                all_indexed_chunks.extend(chunks);
            }, 
            Err(e) => {
                return Err(actix_web::error::ErrorInternalServerError(json!({
                    "error": e.to_string()
                })));
            }
        }
    }

    Ok(HttpResponse::Ok()
    .insert_header(("X-Session-Id", session_id.clone())) // Add session_id in custom header
    .json(json!({
        "session_id": session_id,
        "message": "Request processed successfully",
        "chunks_length": all_indexed_chunks.len()
    })))

}

#[derive(Deserialize)]
struct QueryParams {
    session_id: String,
    user_id: Option<String>,
}

#[get("/rags/index/code")]
async fn get_indexed_context(query: web::Query<QueryParams>) -> Result<HttpResponse, Error>  {
    let session_id = &query.session_id;
    let user_id = query.user_id.as_deref().unwrap_or("user_id");
    let entries = DB_INSTANCE.fetch_session_context(user_id, session_id);

    Ok(HttpResponse::Ok()
    .insert_header(("X-Session-Id", session_id.clone())) // Add session_id in custom header
    .json(json!({
        "session_id": session_id,
        "message": "Request processed successfully",
        "files": entries
    })))
}