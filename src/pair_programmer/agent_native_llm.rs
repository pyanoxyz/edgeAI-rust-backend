

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct NativeLLMAgent {
    user_prompt: String,
    prompt_with_context: String,
}

impl NativeLLMAgent {
    pub fn new(user_prompt: String, prompt_with_context: String) -> Self {
        NativeLLMAgent {
            user_prompt,
            prompt_with_context,
        }
    }
}
#[async_trait]
impl Agent for NativeLLMAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "system-code";
        return name.to_string()    }

    fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
            ou are an AI pair programmer executing steps in a complex programming problem. 
        Use **re-reading and reflection** to optimize your approach for each step, while maintaining context from recent work.

        Your Approach:
        1. **Focus on the current step** while considering recent steps and relevant context.
        2. **Re-read previous steps** to maintain accuracy and continuity, avoiding redundant work.
        3. Generate code that builds upon the recent execution, maintaining **consistency** with coding styles and patterns.
        4. **Anticipate upcoming steps** when necessary to improve efficiency.

        Output Format:
        **Code Implementation**:
        [Your code here]

        **Explanation**:
        [Your detailed explanation, keeping it minimal and tied to the current step]

        Stay focused on the current_step while remaining aware of the overall_context and progress.
        If the steps has already been excuted in the previous steps, skip repeating code.
        If you think that the step is redudant, Feel free to skip the step.
        Consider the **recent_discussion** by the user before proceeding. Avoid unnecessary comments and **do not suggest or provide Next Steps**.

        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}