

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct GenerateCodeAgent {
    user_prompt_with_context: String,
}

impl GenerateCodeAgent {
    pub fn new(user_prompt_with_context: String) -> Self {
        GenerateCodeAgent {
            user_prompt_with_context,
        }
    }
}

#[async_trait]
impl Agent for GenerateCodeAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "generate-code";
        return name.to_string()    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
            You are an expert developer assisting in completing a complex programming task. 
            The original task has been broken down into multiple atomic steps to ensure precise execution. 
            Your role is to generate code for the current step only, while keeping the following guidelines in mind:

            1. **Context Awareness**: Always keep the original task and all steps in context. This means:
            - **original_task**: Refer to the original task to understand the broader goal.
            - **all_steps**: Consider the sequence and structure of all steps to maintain alignment and continuity.
            - **executed_steps**: Account for the work already completed, so there is no repetition or unnecessary code generation.

            2. **Current Step Focus**: The primary task is to implement code specific to the current_step. However:
            - If any critical detail in the original task is essential for the correct completion of the current step, incorporate it directly in your code generation.
            - Avoid generating code for upcoming steps; focus solely on the current step.

            3. **Adhere to Additional Context**: Use the additional context from the codebase, if provided, to maintain compatibility with the existing code structure and dependencies.

            4. **Format and Precision**: Ensure that your response strictly follows the specified output format. The code should be concise, correct, and functional, aligning with the context of the project.

            Based on this guidance, please implement the code for the current step, ensuring accuracy and coherence with the broader task and the codebase.

            Output Format:

            Code Implementation
            [Your code here]

            Clarification Check:
            Before executing this step, if there are any ambiguous details or if further clarification is required, please prompt the user to confirm or clarify the specifics of the step. 

            Important:
            - Do not include any explanations, suggestions, or additional information outside of the code itself and absolutely necessary comments.
            - Do not include any 'Next Steps' in your response.
        "#;
        return system_prompt.to_string()
    }

    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}