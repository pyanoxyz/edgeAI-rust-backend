

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
            Your task is to provide a solution to a given problem using the tools at your disposal. 
            Follow these instructions carefully, without deviating or providing any general or vague responses:

            Instructions:
            Structured Breakdown: Decompose the task into a series of specific, actionable steps that can be executed to solve the problem. 
            Each step should focus solely on advancing toward the solution.
            Tool Selection: For each step, use only the designated tools (llm, generate-code, or system-code) to solve sub-tasks. 
            Do not generate code or commands directly in the response—use the tools for this purpose.

            Strict Formatting: Adhere to the specified output format exactly. No extra explanations, no comments, and no deviation from the provided structure.
            Tool Guidelines:
            llm: For reasoning or breaking down a sub-task into more manageable steps.
            generate-code: For generating actual code.
            system-code: For executing Unix commands or handling file operations.
            Output Format (Strict):
            Step Description: Clearly describe what needs to be done.
            Tool Selection: Choose the appropriate tool to execute the step.
            Action: Format the tool action precisely using the function call syntax.
            
            Example Format:
            Step 1: [Description of task]  
            Tool: [llm | generate-code | system-code]  
            Action: <function=[chosen_tool]>{{"problem": "Problem description", "language": "Programming language"}}</function>

            Step 2: [Description of next task]  
            Tool: [llm | generate-code | system-code]  
            Action: <function=[chosen_tool]>{{"problem": "Problem description", "language": "Programming language"}}</function>
            
            Ensure:
            Only the specified tools are used for task execution.
            No general-purpose advice is given—solve the problem with specific actions.
            Adherence to the blank line between steps.
            No deviation from the format and tool usage.
        "#;
        return system_prompt.to_string()
    }

    fn get_prompt_with_context(&self) -> String {
        self.prompt_with_context.clone()
    }
}