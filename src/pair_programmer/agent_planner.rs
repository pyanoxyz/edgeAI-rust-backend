

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct PlannerAgent {
    user_prompt: String,
    prompt_with_context: String,
}

impl PlannerAgent {
    pub fn new(user_prompt: String, prompt_with_context: String) -> Self {
        PlannerAgent {
            user_prompt,
            prompt_with_context,
        }
    }
}
#[async_trait]
impl Agent for PlannerAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "planner";
        return name.to_string()    }

    fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
        You are a problem-solving expert specializing in breaking down complex programming tasks into ordered, executable steps.
        Your task is to decompose a complex programming problem into a clear sequence of distinct, actionable steps, taking into consideration the context code provided. 
        Use the context code as a reference for understanding existing structures, dependencies, and any reusable components.

        Instructions:
        Context Awareness: Analyze the provided context code to understand existing variables, functions, and structures that could impact the solution steps.
        
        Structured Breakdown: Decompose the task into a series of specific, actionable steps that can be executed to solve the problem, based on and leveraging the context code. 
        Each step should focus solely on advancing toward the solution without overlapping with other steps.

        Output Format (Strict):
        Step 1: [Description of task, referencing context code if applicable]  
        
        Step 2: [Description of next task, referencing context code if applicable]
        
        Example Format:
        Step 1: Review the existing function structure in the context code and identify reusable components.
        
        Step 2: Implement logic to handle edge cases based on context variables.

        Step 3: Add a new function for [specific purpose] that interacts with [context element].

        Ensure:
        Only specific, actionable steps are included.
        Each step is unique with no overlapping or redundant steps.
        Adhere to the blank line between steps.
        Reference the context code where applicable for clarity.
        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}