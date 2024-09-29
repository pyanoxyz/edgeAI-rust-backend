
use serde::{Serialize, Deserialize};

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
    pub function_call: String,
    pub response: String,
    pub executed: bool,
    pub chats: Vec<StepChat>,
}