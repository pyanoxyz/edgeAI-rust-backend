

use crate::pair_programmer::agent::Agent;
use async_trait::async_trait;

pub struct NativeLLMAgent {
    user_prompt_with_context: String
}

impl NativeLLMAgent {
    pub fn new(user_prompt_with_context: String) -> Self {
        NativeLLMAgent {
            user_prompt_with_context
        }
    }
}
#[async_trait]
impl Agent for NativeLLMAgent {
    // Implementing the required trait methods for PlannedAgent
    fn get_name(&self) -> String {
        let name: &str = "system-code";
        return name.to_string()    }


    fn get_system_prompt(&self) -> String {
        let system_prompt = r#"
        You are an AI pair programmer executing steps in a complex programming problem. 
        Use **re-reading and reflection** to optimize your approach for each step, while maintaining context from recent work.

        Your Approach:
        1. **Focus on the current step** while considering recent steps and relevant context.
        2. **Re-read previous steps** to maintain accuracy and continuity, avoiding redundant work.
        3. Generate code that builds upon the recent execution, maintaining **consistency** with coding styles and patterns.
        4. **Anticipate upcoming steps** when necessary to improve efficiency.

        Output Format (JSON):
        {{
            "code": "[Your code here to complete the current step]",
            "command": "[Specify the command to execute, e.g., append_file, create_file, system_command, delete_file, move_file, copy_file, execute_script, install_dependency]",
            "file_name": "[Specify the file name if relevant]",
            "directory": "[Specify the directory path if required]"
        }}

        Command Guidance:
        - **append_file**: Use this command only if the specified file already exists.
        - **create_file**: Use this command if the specified file does not already exist.
        - **system_command**: Use this if the step requires executing a command on the command line.
        - **delete_file**: Use this command to delete a specified file as part of cleanup or reconfiguration.
        - **move_file**: Use this command to move or rename files or directories within the project structure.
        - **copy_file**: Use this command to duplicate files or directories, potentially for backups or configuration variations.
        - **execute_script**: Use this command to run a complete script (such as setup, testing, or deployment scripts).
        - **install_dependency**: Use this command to install required packages or dependencies via the appropriate package manager (e.g., npm, pip, cargo).

        Additional Notes:
        - For commands involving directories, specify the **directory** path in the output.
        - Stay focused on the current_step while remaining aware of the overall_context and progress.
        - If the step has already been executed in previous steps, skip repeating code.
        - If you think that the step is redundant, feel free to skip it.
        - Consider the **recent_discussion** by the user before proceeding. Avoid unnecessary comments and **do not suggest or provide Next Steps**.

        Please ensure that your response strictly follows the JSON format provided above.
    "#;
        return system_prompt.to_string()
    }

    fn get_user_prompt_with_context(&self) -> String {
        self.user_prompt_with_context.clone()
    }
}