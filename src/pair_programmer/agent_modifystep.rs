use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct ModifyStepAgent {
    user_prompt_with_context: String
}

impl ModifyStepAgent {
    pub fn new(user_prompt_with_context: String) -> Self {
        ModifyStepAgent {
            user_prompt_with_context
        }
    }
}

#[async_trait]
impl Agent for ModifyStepAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "modify-step";
        return name.to_string()    }


    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
            Analyze the TASK in the context of ALL_STEPS, USER_PROMPT, and CONTEXT_CODE.
            Generate a revised TASK that is coherent with ALL_STEPS and incorporates any specific instructions or preferences from USER_PROMPT.
            Ensure the output is a new, refined TASK that aligns with prior steps and remains consistent with the relevant code snippets in CONTEXT_CODE.
        "#;
        return system_prompt.to_string()
    }

    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}

