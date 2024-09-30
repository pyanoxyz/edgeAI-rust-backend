use crate::pair_programmer::{agent_planner::PlannerAgent,
    agent_generate_code::GenerateCodeAgent,
    agent_native_llm::NativeLLMAgent,
    agent_system_code::SystemCodeAgent,
    agent_rethinker::RethinkerAgent,
    agent::Agent};
use async_trait::async_trait;
use actix_web::{HttpResponse, Error as ActixError};

pub enum AgentEnum {
    GenerateCode(GenerateCodeAgent),
    NativeLLM(NativeLLMAgent),
    Planner(PlannerAgent),
    Rethinker(RethinkerAgent),
    SystemCode(SystemCodeAgent),
}


impl AgentEnum {
    pub fn new(agent_type: &str, user_prompt: String, prompt_with_context: String) -> Result<Self, ActixError> {
        match agent_type {
            "generate-code" => Ok(AgentEnum::GenerateCode(GenerateCodeAgent::new(user_prompt, prompt_with_context))),
            "system-code" => Ok(AgentEnum::SystemCode(SystemCodeAgent::new(user_prompt, prompt_with_context))),
            "llm" => Ok(AgentEnum::NativeLLM(NativeLLMAgent::new(user_prompt, prompt_with_context))),
            "planner" => Ok(AgentEnum::Planner(PlannerAgent::new(user_prompt, prompt_with_context))),
            "rethinker" => Ok(AgentEnum::Rethinker(RethinkerAgent::new(user_prompt, prompt_with_context))),
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
        }
    }

    fn get_user_prompt(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_user_prompt(),
            AgentEnum::NativeLLM(agent) => agent.get_user_prompt(),
            AgentEnum::Planner(agent) => agent.get_user_prompt(),
            AgentEnum::Rethinker(agent) => agent.get_user_prompt(),
            AgentEnum::SystemCode(agent) => agent.get_user_prompt(),
        }
    }

    fn get_system_prompt(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_system_prompt(),
            AgentEnum::NativeLLM(agent) => agent.get_system_prompt(),
            AgentEnum::Planner(agent) => agent.get_system_prompt(),
            AgentEnum::Rethinker(agent) => agent.get_system_prompt(),
            AgentEnum::SystemCode(agent) => agent.get_system_prompt(),
        }
    }

    fn get_prompt_with_context(&self) -> String {
        match self {
            AgentEnum::GenerateCode(agent) => agent.get_prompt_with_context(),
            AgentEnum::NativeLLM(agent) => agent.get_prompt_with_context(),
            AgentEnum::Planner(agent) => agent.get_prompt_with_context(),
            AgentEnum::Rethinker(agent) => agent.get_prompt_with_context(),
            AgentEnum::SystemCode(agent) => agent.get_prompt_with_context(),
        }
    }

    async fn execute(&self, user_id: &str, session_id: &str, pair_programmer_id: &str) -> Result<HttpResponse, ActixError> {
        match self {
            AgentEnum::GenerateCode(agent) => agent.execute(user_id, session_id, pair_programmer_id).await,
            AgentEnum::NativeLLM(agent) => agent.execute(user_id, session_id, pair_programmer_id).await,
            AgentEnum::Planner(agent) => agent.execute(user_id, session_id, pair_programmer_id).await,
            AgentEnum::Rethinker(agent) => agent.execute(user_id, session_id, pair_programmer_id).await,
            AgentEnum::SystemCode(agent) => agent.execute(user_id, session_id, pair_programmer_id).await,
        }
    }
}
