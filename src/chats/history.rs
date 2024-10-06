use actix_web::{ get, web, HttpResponse, Error };
use serde::Deserialize;
use serde_json::json;
use crate::database::db_config::DB_INSTANCE;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(session_chat_history)
        .service(request_type_chat_history)
        .service(whole_chat_history); // Register the correct route handler
}

// The correct handler for GET steps
#[get("/chat/history/all")]
async fn whole_chat_history() -> Result<HttpResponse, Error> {
    // Use into_inner to get the inner String from the Path extractors

    // Fetch the steps for the provided pair_programmer_id
    let steps = DB_INSTANCE.fetch_chats_all();

    // Return the result as JSON
    Ok(HttpResponse::Ok().json(steps))
}

#[derive(Deserialize)]
struct SessionParams {
    page: Option<u32>,
    page_size: Option<u32>,
}
// The correct handler for GET steps
#[get("/chat/history/session_id/{session_id}")]
async fn session_chat_history(
    path: web::Path<String>,
    query: web::Query<SessionParams>
) -> Result<HttpResponse, Error> {
    // Use into_inner to get the inner String from the Path extractors
    let session_id = path.into_inner();

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let skip = (page - 1) * page_size;
    // Fetch the steps for the provided pair_programmer_id
    let history = DB_INSTANCE.fetch_chats_for_session(&session_id, skip, page_size);

    let response =
        json!({
        "result" : history,
        "total": history.len(),
        "page": page,
        "page_size": page_size,
        "session_id": session_id
    });
    // Return the result as JSON
    Ok(HttpResponse::Ok().json(response))
}

#[derive(Deserialize)]
struct SessionTypeParams {
    page: Option<u32>,
    page_size: Option<u32>,
}

// The correct handler for GET steps
#[get("/chat/history/request_type/{request_type}")]
async fn request_type_chat_history(
    path: web::Path<String>,
    query: web::Query<SessionTypeParams>
) -> Result<HttpResponse, Error> {
    // Use into_inner to get the inner String from the Path extractors
    let request_type = path.into_inner();
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let skip = (page - 1) * page_size;
    let history = DB_INSTANCE.fetch_chats_for_request_type(&request_type, skip, page_size);

    let response =
        json!({
        "result" : history,
        "total": history.len(),
        "page": page,
        "page_size": page_size
    });

    // Return the result as JSON
    Ok(HttpResponse::Ok().json(response))
}
