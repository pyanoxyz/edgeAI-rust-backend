use actix_web::{ post, get, web, delete, HttpRequest, HttpResponse, Error };
use serde::{ Deserialize, Serialize };
use crate::authentication::authorization::is_request_allowed;
use log::{ info, warn };
use crate::session_manager::check_session;
use serde_json::json;
use crate::rag::code_rag::index_code;
use crate::parser::parse_code::Chunk;
use crate::database::db_config::DB_INSTANCE;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::similarity_index::index::{search_index, remove_from_index};

#[derive(Debug, Serialize, Deserialize)]
pub struct RagRequest {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub files: Option<Vec<String>>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(rag_request)
        .service(get_indexed_context)
        .service(fetch_similar_entries)
        .service(delete_rag_context); // Register the correct route handler
}

#[post("/rags/index/code")]
pub async fn rag_request(
    data: web::Json<RagRequest>,
    req: HttpRequest
) -> Result<HttpResponse, Error> {
    info!("Session_id = {:?}", data.session_id.clone());
    info!("Files = {:?}", data.files.clone());

    // Check session and extract user ID from the request
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

    let user_id: String = match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => user.user_id.clone(),
        Ok(None) => data.user_id.clone().unwrap_or_else(|| "user_id".to_string()), // Set default user_id if not present
        Err(_) => {
            // Handle the error case by propagating the HttpResponse error
            return Err(
                actix_web::error::ErrorInternalServerError(
                    json!({ "error": "An error occurred during request validation" })
                )
            );
        }
    };

    let entries: Vec<serde_json::Value> = DB_INSTANCE.fetch_session_context_files(&user_id, &session_id);
    let indexed_paths: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.get("path").and_then(|p| p.as_str().map(String::from)))
        .collect();


    let mut all_indexed_chunks: Vec<Chunk> = Vec::new();
    // Iterate over the files and call `index_code` for each
    for file_path in &data.files {
        if indexed_paths.contains(file_path) {
            warn!("Skipping already indexed file: {:?}", file_path);
            continue;
        }

        match index_code(&user_id, &session_id, file_path).await {
            Ok(chunks) => {
                all_indexed_chunks.extend(chunks);
            }
            Err(e) => {
                return Err(
                    actix_web::error::ErrorInternalServerError(json!({ "error": e.to_string() }))
                );
            }
        }
    }

    let data =
        json!({
            "message": { "session_id": session_id, "indexed_files": data.files}
        });

    Ok(
        HttpResponse::Ok()
            .insert_header(("X-Session-Id", session_id.clone())) // Add session_id in custom header
            .json(data)
    )
}

#[delete("/rags/index/code")]
pub async fn delete_rag_context(
    data: web::Json<DeleteRequest>,
    req: HttpRequest
) -> Result<HttpResponse, Error> {
    // Check if session_id, user_id, or files are missing or empty
    if
        data.session_id
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
    {
        return Ok(
            HttpResponse::BadRequest().json(
                json!({ "error": "session_id is required and cannot be empty" })
            )
        );
    }

    if
        data.files
            .as_ref()
            .map(|f| f.is_empty())
            .unwrap_or(true)
    {
        return Ok(
            HttpResponse::BadRequest().json(
                json!({ "error": "files array is required and cannot be empty" })
            )
        );
    }

    info!("Session_id = {:?}", data.session_id.clone());
    info!("Files = {:?}", data.files.clone());

    // Check session and extract user ID from the request
    let session_id = match check_session(data.session_id.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(
                actix_web::error::ErrorInternalServerError(json!({ "error": e.to_string() }))
            );
        }
    };

    let user_id: String = match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => user.user_id.clone(),
        Ok(None) => data.user_id.clone().unwrap_or_else(|| "user_id".to_string()), // Set default user_id if not present
        Err(_) => {
            // Handle the error case by propagating the HttpResponse error
            return Err(
                actix_web::error::ErrorInternalServerError(
                    json!({ "error": "An error occurred during request validation" })
                )
            );
        }
    };

    // Iterate over the files and call `delete_indexed_code` for each file path
    for file_path in data.files.as_ref().unwrap() {
        match DB_INSTANCE.delete_parent_context(file_path) {
            Ok(_) => {
                info!("Successfully deleted parent context for file: {:?}", file_path);
            }
            Err(e) => {
                return Err(
                    actix_web::error::ErrorInternalServerError(json!({"error": e.to_string() }))
                );
            }
        }

        let vec_row_ids = match DB_INSTANCE.delete_children_context_by_parent_path(&user_id, &session_id, file_path) {
            Ok(ids) => {
                info!("Successfully deleted chunks for file: {}", file_path);
                ids
            }
            Err(e) => {
                return Err(
                    actix_web::error::ErrorInternalServerError(json!({ "error": e.to_string() }))
                );
            }
        };
        info!("vec_row_ids that awere deleted from sqlite {:?}", vec_row_ids);
        remove_from_index(&session_id, vec_row_ids);
    }

    Ok(
        HttpResponse::Ok()
            .insert_header(("X-Session-Id", session_id.clone())) // Add session_id in custom header
            .json(json!({ "message": "Context successfully deleted" }))
    )
}

#[derive(Deserialize)]
struct QueryParams {
    session_id: Option<String>, // Make session_id an Option to handle missing field
    user_id: Option<String>,
}

#[get("/rags/index/code")]
async fn get_indexed_context(query: web::Query<QueryParams>) -> Result<HttpResponse, Error> {
    // Check if session_id is empty and return an error if it is
    if query.session_id.is_none() {
        return Ok(
            HttpResponse::BadRequest().json(
                json!({
                "error": "session_id is required"
            })
            )
        );
    }

    let session_id = query.session_id.as_ref().unwrap(); // Safe
    let user_id = query.user_id.as_deref().unwrap_or("user_id");
    let entries = DB_INSTANCE.fetch_session_context_files(user_id, session_id);

    Ok(
        HttpResponse::Ok()
            .insert_header(("X-Session-Id", session_id.clone())) // Add session_id in custom header
            .json(json!({ "data": entries }))
    )
}

#[derive(Deserialize)]
struct FetchContextRequest {
    session_id: String,
    query: String,
}

#[post("/rags/index/fetch-context")]
async fn fetch_similar_entries(
    data: web::Json<FetchContextRequest>,
    _req: HttpRequest
) -> Result<HttpResponse, Error> {
    if data.session_id.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(
                serde_json::json!({
            "detail": "context Id is required"
        })
            )
        );
    }

    if data.query.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(
                serde_json::json!({
            "detail": "query is required"
        })
            )
        );
    }

    let query = &data.query;

    let embeddings_result = generate_text_embedding(&query).await;
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

    let chunk_ids = search_index(&data.session_id, query_embeddings.clone(), 10);

    let entries = DB_INSTANCE.get_row_ids(chunk_ids).unwrap();
    info!("All the matching entries {:?}", entries);


    Ok(
        HttpResponse::Ok()
            .insert_header(("X-Session-Id", data.session_id.clone())) // Add session_id in custom header
            .json(
                json!({
        "session_id": data.session_id.clone(),
        "message": "Request processed successfully",
        "result": entries
    })
            )
    )
}