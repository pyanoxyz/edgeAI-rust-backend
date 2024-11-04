use crate::pair_programmer::{agent_planner::PlannerAgent,
    agent_generate_code::GenerateCodeAgent,
    agent_native_llm::NativeLLMAgent,
    agent_system_code::SystemCodeAgent,
    agent_rethinker::RethinkerAgent,
    agent_chat::ChatAgent,
    agent::Agent};
use async_trait::async_trait;
use actix_web::Error as ActixError;
use crate::llm_stream::types::AccumulatedStream;
use reqwest::Client;
pub enum AgentEnum {
    GenerateCode(Box<dyn Agent>),
    NativeLLM(Box<dyn Agent>),
    Planner(Box<dyn Agent>),
    Rethinker(Box<dyn Agent>),
    SystemCode(Box<dyn Agent>),
    Chat(Box<dyn Agent>)    
}


impl AgentEnum {
    pub fn new(agent_type: &str, user_prompt_with_context: String) -> Result<Self, ActixError> {
        match agent_type {
            "generate-code" => Ok(AgentEnum::GenerateCode(Box::new(GenerateCodeAgent::new(user_prompt_with_context)))),
            "system-code" => Ok(AgentEnum::SystemCode(Box::new(SystemCodeAgent::new(user_prompt_with_context)))),
            "llm" => Ok(AgentEnum::NativeLLM(Box::new(NativeLLMAgent::new(user_prompt_with_context)))),
            "planner" => Ok(AgentEnum::Planner(Box::new(PlannerAgent::new(user_prompt_with_context)))),
            "rethinker" => Ok(AgentEnum::Rethinker(Box::new(RethinkerAgent::new(user_prompt_with_context)))),
            "chat" => Ok(AgentEnum::Chat(Box::new(ChatAgent::new(user_prompt_with_context)))),       
            _ => Err(actix_web::error::ErrorInternalServerError(format!("Unknown agent type: {}", agent_type)).into()),
        }
    }
}
#[async_trait]
impl Agent for AgentEnum {

    fn get_name(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_name(),
            AgentEnum::NativeLLM(agent) => agent.get_name(),
            AgentEnum::Planner(agent) => agent.get_name(),
            AgentEnum::Rethinker(agent) => agent.get_name(),
            AgentEnum::SystemCode(agent) => agent.get_name(),
            AgentEnum::Chat(agent) => agent.get_name(),
        }
    }

    fn get_user_prompt_with_context(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_user_prompt_with_context(),
            AgentEnum::NativeLLM(agent) => agent.get_user_prompt_with_context(),
            AgentEnum::Planner(agent) => agent.get_user_prompt_with_context(),
            AgentEnum::Rethinker(agent) => agent.get_user_prompt_with_context(),
            AgentEnum::SystemCode(agent) => agent.get_user_prompt_with_context(),
            AgentEnum::Chat(agent) => agent.get_user_prompt_with_context()
            
        }
    }

    fn get_system_prompt(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_system_prompt(),
            AgentEnum::NativeLLM(agent) => agent.get_system_prompt(),
            AgentEnum::Planner(agent) => agent.get_system_prompt(),
            AgentEnum::Rethinker(agent) => agent.get_system_prompt(),
            AgentEnum::SystemCode(agent) => agent.get_system_prompt(),
            AgentEnum::Chat(agent) => agent.get_system_prompt()
        }
    }


    async fn execute(&self, client: &Client) -> Result<AccumulatedStream, ActixError> {
        match self {
            AgentEnum::GenerateCode(agent) => agent.execute(&client).await,
            AgentEnum::NativeLLM(agent) => agent.execute(&client).await,
            AgentEnum::Planner(agent) => agent.execute(&client).await,
            AgentEnum::Rethinker(agent) => agent.execute(&client).await,
            AgentEnum::SystemCode(agent) => agent.execute(&client).await,
            AgentEnum::Chat(agent) => agent.execute(&client).await
        }
    }
}