
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
    pub step_id: String,
    pub pair_programmer_id: String,
    pub heading: String,
    pub tool: String,
    pub action: String,
    pub response: String,
    pub executed: bool,
    pub chats: Vec<StepChat>,
}