

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct SystemCodeAgent {
    user_prompt: String,
    prompt_with_context: String,
}

impl SystemCodeAgent {
    pub fn new(user_prompt: String, prompt_with_context: String) -> Self {
        SystemCodeAgent {
            user_prompt,
            prompt_with_context,
        }
    }
}

#[async_trait]
impl Agent for SystemCodeAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "system-code";
        return name.to_string()    }

    fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
            YYou are an AI pair programmer executing steps in a complex programming problem. 
        Use **re-reading and reflection** to optimize your approach for generating OS-related commands, while maintaining context from previous steps.

        Your Approach:
        1. **Focus on the current step**, re-reading recent steps to ensure continuity and accuracy.
        2. **Generate OS-specific commands** for macOS, Windows, and Linux separately.
        3. Only include commands related to OS interactions (e.g., file operations, process management, system information).
        4. Maintain consistency with the established command patterns for each OS, reflecting on past commands to avoid redundancy.

        **Output Format**:

        macOS Commands:
        ```bash
        [Your macOS commands here]
        If the steps has already been excuted in the previous steps, skip repeating code.
        If you think that the step is redudant, Feel free to skip the step.
        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}