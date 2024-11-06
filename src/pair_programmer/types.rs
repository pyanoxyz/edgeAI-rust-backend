
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct StepData {
    pub step_number: usize,
    pub task_heading: String,
    pub function_call: String,
    pub step_chat: String,
    pub all_steps: String,
    pub steps_executed_response: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StepDetails {
    filename: Option<String>,
    directory: Option<String>,
    command: Option<String>,
    package_name: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Step {
    pub step_number: String,
    pub heading: String,
    pub action: String,
    pub details: StepDetails,
}

#[derive(Deserialize, Debug)]
pub struct StepsWrapper {
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StepChat{
    pub prompt: String,
    pub response: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PairProgrammerStep {
    pub step_number: usize,
    pub heading: String,
    pub tool: String,
    pub response: String,
    pub executed: bool,
    pub chats: Vec<StepChat>,
}

impl PairProgrammerStep {
    // Constructor for StepData to simplify creation
    pub fn new(
        step_number: usize,
        heading: String,
        tool: String,
        response: String,
        executed: bool,
        chats: Vec<StepChat>,
   
    ) -> Self {
        Self {
            step_number,
            heading,
            tool,
            response,
            executed,
            chats
        }
    }
}