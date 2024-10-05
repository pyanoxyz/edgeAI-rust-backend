use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::StreamExt; // Ensure StreamExt is imported
use actix_web::Error as ActixError;
use crate::platform_variables::get_default_prompt_template;
use crate::utils::is_cloud_execution_mode;
use crate::llm_stream::local::local_agent_execution;
use crate::llm_stream::remote::remote_agent_execution;
use crate::llm_stream::types::AccumulatedStream;



#[async_trait]
pub trait Agent: Send + Sync {

    fn get_prompt(&self) -> String {
        let llm_prompt_template = get_default_prompt_template();
        llm_prompt_template
            .replace("{system_prompt}", &self.get_system_prompt())
            .replace("{user_prompt}", &self.get_user_prompt())
    }

    async fn execute(&self) -> Result<AccumulatedStream, ActixError> {
        let _prompt = self.get_prompt();

        let stream: AccumulatedStream = if is_cloud_execution_mode() {
            remote_agent_execution(&self.get_system_prompt(), &self.get_prompt_with_context())
                .await
                .map_err(|e| ActixError::from(actix_web::error::ErrorInternalServerError(e.to_string())))?
        } else {
            local_agent_execution(&self.get_system_prompt(), &self.get_prompt_with_context())
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

    fn to_string(&self) -> String {
        format!("Agent(name='{}')", self.get_name())
    }

    // Helper methods that concrete types must implement
    fn get_name(&self) -> String;
    fn get_user_prompt(&self) -> String;
    fn get_system_prompt(&self) -> String;
    fn get_prompt_with_context(&self) -> String;
}