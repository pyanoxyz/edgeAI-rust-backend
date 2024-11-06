

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
        
          You are a problem-solving expert specializing in breaking down complex programming tasks into ordered, executable steps. Your task is to decompose a complex programming problem into a clear sequence of distinct, actionable steps in strict JSON format, referencing filenames and directory names from the provided `context_code`.

          Instructions:
          - **Context Awareness**: Analyze the provided `context_code` to understand existing file structures, directory organization, variables, functions, and any reusable components that could impact the solution steps.
          - **Command Selection for Actions**: Only use commands explicitly listed in the `command_guidance` for the `action` field in each step. Each command should match the required action for the step, as specified in `command_guidance`. Do not invent new commands or deviate from this list.
          - **Structured Breakdown**: Divide the complex problem into the smallest possible steps that can be executed independently in a JSON structure. Each step should contain a unique ID, a description, an action (using only commands listed in `command_guidance`), and relevant details, such as filenames, directory names, and specific parameters needed to execute the step. Each step should be actionable and self-contained, leveraging filenames and directory names from the `context_code`.

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
                "package_name": "[packages to be installed if applicable]"
                "command": "[Command to be executed if applicable]"
              }
            },
            ...
          ]
        }

      command_guidance (for action field in each step):

        - edit_file: Use this command only if the specified file already exists.
        - create_file: Use this command if the specified file does not already exist.
        - system_command: Use this command if the step requires executing a command on the command line.
        - delete_file: Use this command to delete a specified file as part of cleanup or reconfiguration.
        - move_file: Use this command to move or rename files or directories within the project structure.
        - copy_file: Use this command to duplicate files or directories, potentially for backups or configuration variations.
        - execute_script: Use this command only when a full script file exists and has been previously created as a step; do not use this action unless all necessary scripts are already written.
        - install_dependency: Use this command to install required packages or dependencies via the appropriate package manager (e.g., npm, pip, cargo).
        - create_directory: Use this command to create a new directory.
        - remove_directory: Use this command to remove an existing directory as part of cleanup.
        - modify_config: Use this command to change configurations in a specified file.
        - run_tests: Use this command to execute tests within the project.


      Ensure:
      - **Each step is represented as a JSON object within the "steps" array.**
      - **Each step contains a unique ID, a clear description, a specific action type, and relevant details (filename, directory, command) only.**
      - **All necessary scripts must have creation steps if they are required in the solution.**
      - **Output only the number of steps necessary to solve the problem, with no upper limit.**
      - DO NOT hallucinate actions or scripts, and MUST ADHERE TO THE OUTPUT FORMAT strictly.
      - MUST NOT provide any comments, JUST PURE JSON.
      - AVOID trailing comma in the end ERROR
      
        "#;
        return system_prompt.to_string()
    }

    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}