

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct PlannerAgent {
    user_prompt_with_context: String,
}

impl PlannerAgent {
    pub fn new(user_prompt_with_context: String) -> Self {
        PlannerAgent {
            user_prompt_with_context,
        }
    }
}
#[async_trait]
impl Agent for PlannerAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "planner";
        return name.to_string()    }


    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
        You are a problem-solving expert specializing in breaking down complex programming tasks into ordered, executable steps.
        Your task is to decompose a complex programming problem into a clear sequence of distinct, actionable steps in strict JSON format, 
        referencing filenames and directory names from the provided context_code in maximum 10 steps.

        Instructions:
      Context Awareness: Analyze the provided context_code to understand existing file structures, directory organization, variables, functions, and any reusable components that could impact the solution steps.
      Command Selection: Only use commands explicitly listed in the command_guidance. Do not invent new commands or deviate from this list.
      Structured Breakdown: Decompose the task into a JSON structure where each step contains a unique ID, description, action, and relevant details such as filenames, directory names, and commands needed to execute the step. Each step should be actionable and self-contained, leveraging filenames and directory names from the context_code.

        Output Format (Strict VALIDATED JSON with all keys as strings and without trailing commas):
        {
          "steps": [
            {
              "step_number": "1",
              "heading": "[Description of task, referencing specific filenames and/or directory names if applicable]",
              "action": "[e.g., create_directory, open_file, run_command]",
              "details": {
                "filename": "[filename if applicable]",
                "directory": "[directory if applicable]",
                "command": "[command to execute if applicable]"
              }
            },
            ...
          ]
        }


      Command Guidance:
      - **edit_file**: Use this command only if the specified file already exists.
      - **create_file**: Use this command if the specified file does not already exist.
      - **system_command**: Use this if the step requires executing a command on the command line.
      - **delete_file**: Use this command to delete a specified file as part of cleanup or reconfiguration.
      - **move_file**: Use this command to move or rename files or directories within the project structure.
      - **copy_file**: Use this command to duplicate files or directories, potentially for backups or configuration variations.
      - **execute_script**: Use this command to run a complete script (such as setup, testing, or deployment scripts).
      - **install_dependency**: Use this command to install required packages or dependencies via the appropriate package manager (e.g., npm, pip, cargo).

      Ensure:
      - Each step is represented as a JSON object within the \"steps\" array.
      - Each step contains a unique ID, a clear description, a specific action type, and relevant details (filename, directory, command).
      - No code snippets should be provided.
      - MUST NOT providfe any comments , JUST PURE JSON
      - **Output only the number of steps necessary to solve the problem, up to a maximum of 10 steps.**"

        "#;
        return system_prompt.to_string()
    }

    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}