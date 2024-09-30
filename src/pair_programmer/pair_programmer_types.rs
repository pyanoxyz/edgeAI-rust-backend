
use serde::{Serialize, Deserialize};
use futures::{Stream, StreamExt}; // Ensure StreamExt is imported


#[derive(Serialize, Deserialize, Debug)]
pub struct Step {
    pub step_number: u32,
    pub tool: String,
    pub action: String,
    pub heading: String
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