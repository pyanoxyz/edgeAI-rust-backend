use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::StreamExt; // Ensure StreamExt is imported
use actix_web::Error as ActixError;
use crate::utils::is_cloud_execution_mode;
use crate::llm_stream::local::local_agent_execution;
use crate::llm_stream::remote::remote_agent_execution;
use crate::llm_stream::types::AccumulatedStream;

use reqwest::Client;


#[async_trait]
pub trait Agent: Send + Sync {

    async fn execute(&self, client: &Client) -> Result<AccumulatedStream, ActixError> {

        let stream: AccumulatedStream = if is_cloud_execution_mode() {
            remote_agent_execution(&self.get_system_prompt(), &self.get_user_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
        } else {
            local_agent_execution(&client, &self.get_system_prompt(), &self.get_user_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
        };

        let accumulated_content = Arc::new(Mutex::new(String::new()));
        let accumulated_content_clone = Arc::clone(&accumulated_content);

        let accumulated_stream = stream.inspect(move |chunk_result| {
            if let Ok(chunk) = chunk_result {
                if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                    let mut accumulated = accumulated_content_clone.lock().unwrap();
                    accumulated.push_str(chunk_str);
                }
            }
        });

        Ok(Box::pin(accumulated_stream))
    }

    // fn to_string(&self) -> String {
    //     format!("Agent(name='{}')", self.get_name())
    // }

    // Helper methods that concrete types must implement
    fn get_name(&self) -> String;
    fn get_system_prompt(&self) -> String;
    fn get_user_prompt_with_context(&self) -> String;
}