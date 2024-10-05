use actix_web::{HttpRequest, HttpResponse};
use serde_json::json;
use std::collections::HashMap;
use crate::utils::is_cloud_execution_mode;
use log::debug;
use actix_web::HttpMessage;  // <-- Bring the HttpMessage trait into scope
// Structs to represent the user and subscription details
#[derive(Clone)]
pub struct User {
    pub user_id: String,
    pub subscription: Subscription,
}

#[derive(Clone)]
pub struct Subscription {
    pub is_trial_active: bool,
    pub pro: ProSubscription,
}

#[derive(Clone)]
pub struct ProSubscription {
    pub active_till: i64,
    pub tokens_left: i64,
}



/// Function that mimics the request validation
pub async fn is_request_allowed(req: HttpRequest) -> Result<Option<User>, HttpResponse> {
    if is_cloud_execution_mode() {        
        log::debug!("Cloud EXECUTION MODE IS ON");

        let headers: HashMap<_, _> = req.headers().iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
            .collect();

        log::debug!("{:?}", headers);

        // Check if API key is present
        if let Some(api_key) = headers.get("api_key") {
            // Simulate database lookup (replace this with actual DB code)
            if let Some(user) = find_user_by_api_key(api_key).await {
                // Check if the trial period is over
                if !user.subscription.is_trial_active {
                    let current_time = current_timestamp(); // Function to get the current time as timestamp

                    // Check subscription validity
                    if current_time > user.subscription.pro.active_till {
                        return Err(HttpResponse::InternalServerError().json(json!({"error": "Please renew your premium subscription"})));
                    }
                    
                    // Check tokens limit
                    if user.subscription.pro.tokens_left < 0 {
                        return Err(HttpResponse::InternalServerError().json(json!({"error": "Please refill your premium subscription, tokens limit has been reached"})));
                    }
                }

                // Store user in the request state (you can use extensions for this)
                req.extensions_mut().insert(user.clone());
                return Ok(Some(user));
            } else {
                return Err(HttpResponse::InternalServerError().json(json!({"error": "API key is not valid"})));
            }
        } else {
            return Err(HttpResponse::InternalServerError().json(json!({"error": "No API key present in the headers"})));
        }
    }

    // If not in cloud mode, assign a default user
    Ok(None)
}

// Mock database lookup (replace with actual DB access)
async fn find_user_by_api_key(api_key: &str) -> Option<User> {
    // Simulate finding the user by API key
    debug!("Implement mongodb retrivela for cloud execution mode for {}", api_key);
    Some(User {
        user_id: "some_user_id".to_string(),
        subscription: Subscription {
            is_trial_active: false,
            pro: ProSubscription {
                active_till: 1700000000,
                tokens_left: 10,
            },
        },
    })
}

// Function to get the current timestamp
fn current_timestamp() -> i64 {
    // Replace with actual timestamp logic
    1600000000
}