use actix_web::{post, web, HttpRequest, HttpResponse, Error};
use serde::{Deserialize, Serialize};
use crate::authentication::authorization::is_request_allowed;
use log::{debug, info};
use crate::session_manager::check_session;
use serde_json::json;
use crate::rag::code_rag::index_code;
use crate::parser::parse_code::Chunk;

#[derive(Debug, Serialize, Deserialize)]
pub struct RagRequest {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub files: Vec<String>, 
}


pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(rag_request); // Register the correct route handler
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

    Ok(HttpResponse::Ok().json(json!({
        "session_id": session_id,
        "message": "Request processed successfully",
        "chunks": all_indexed_chunks
    })))

}

    // // match req {
    // //     Some(req) => {
    // //         if let Ok(Some(user)) = is_request_allowed(req.clone()).await {
    // //             debug!("Ok reached here");

    // //             // Cloud LLM response with actual user ID
    // //             handle_llm_response(
    // //                 Some(req),
    // //                 prompt.system_prompt,
    // //                 &user_prompt,
    // //                 &full_user_prompt,
    // //                 &session_id,
    // //                 &user.user_id,
    // //                 RequestType::Refactor,
    // //             )
    // //             .await
    // //         } else {
    // //             // Local LLM response
    // //             handle_llm_response(
    // //                 None,
    // //                 prompt.system_prompt,
    // //                 &user_prompt,
    // //                 &full_user_prompt,
    // //                 &session_id,
    // //                 "user_id",
    // //                 RequestType::Refactor,
    // //             )
    // //             .await
    // //         }
    // //     }
    // //     None => {
    // //         // Local LLM response without user info
    // //         handle_llm_response(
    // //             None,
    // //             prompt.system_prompt,
    // //             &user_prompt,
    // //             &full_user_prompt,
    // //             &session_id,
    // //             "user_id",
    // //             RequestType::Refactor,
    // //         )
    // //         .await
    // //     }
    // // }

    // }
