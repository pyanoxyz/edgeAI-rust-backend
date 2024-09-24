use crate::database::db_config::DB_INSTANCE;
use crate::authentication::authorization::is_request_allowed;
use actix_web::{error::InternalError, get, web, HttpRequest, HttpResponse, Error};
use log::{debug, error};


pub fn histoy_register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(history); // Register the correct route handler
}


#[get("/history")]
pub async fn history(req: HttpRequest) -> Result<HttpResponse, Error> {
    // Check session and extract user ID from the request
    match is_request_allowed(req.clone()).await {
        Ok(Some(user)) => {
            debug!("Ok reached here");
            let results = DB_INSTANCE.fetch_chats_for_user(&user.user_id);
            Ok(HttpResponse::Ok().json(results))
        }
        Ok(None) => {
            let results = DB_INSTANCE.fetch_chats_for_user("user_id");
            Ok(HttpResponse::Ok().json(results))

        }
        Err(e) => {
            // Error handling with InternalError response
            error!("chat  history fetch failed {:?}", e);
            let err_response = InternalError::from_response("Request failed", e).into();
            Err(err_response)
        }
    }
}
