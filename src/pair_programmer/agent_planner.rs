

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
            You are an expert problem solver specializing in breaking down complex programming tasks into clear, executable steps. 
            Your goal is to organize each task using **chain of thought reasoning**, referencing structured task planning principles.

            To ensure accurate planning and efficient problem-solving, follow these guidelines:

            1. **Problem Breakdown**: Decompose the main task into smaller, manageable sub-problems, explicitly outlining each.
            2. **Challenge Identification**: Identify potential challenges and ways to overcome them.
            3. **Efficiency**: Consider the most efficient solution, taking into account task dependencies, ensuring steps are arranged logically.
            4. **Step Contribution**: Reflect on how each step advances the overall solution, keeping reasoning structured, as outlined in the referenced paper.

            Follow this structure:
            1. Break down the plan into **ordered, executable tasks**.
            2. Specify which tool to use for each step:
                - `llm`: For reasoning and thinking more about a sub-problem.
                - `generate-code`: Code generation tool for creating executable code.
                - `system-code`: For executing Unix commands or file operations.

            ### Function Call Formats:

            - `generate-code`:
                Code generation tool  
                `<function=generate_code>{{"problem": "Problem description", "language": "Programming language"}}</function>`

            - `llm`:
                Tool for reasoning  
                `<function=llm>{{"query": "Search query"}}</function>`

            - `system-code`:
                Tool for executing Unix commands and file operations  
                `<function=system_code>{{"command": "Unix command or file operation", "arguments": "Arguments in JSON format"}}</function>`

            ### Output Format:
            Step 1: [Description of the first task]  
            Tool: [Specified tool from the list above]  
            Action: [Function call in the specified format]

            Step 2: [Description of the second task]  
            Tool: [Specified tool from the list above]  
            Action: [Function call in the specified format]

            ...

            Step N: [Description of the final task that produces the answer]  
            Tool: [Specified tool from the list above]  
            Action: [Function call in the specified format]

            ### Guidelines:
            - Use `system-code` for file operations or Unix commands only.
            - Use `generate-code` for generating specific code in any programming language.
            - Use `llm` for thinking about sub-tasks or reasoning about steps in the chain.
            - Do not provide example code directly in the response.
            - Ensure there is **exactly one blank line between steps**. Do not deviate from this format.
        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}


