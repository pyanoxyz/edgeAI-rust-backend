
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


// #[derive(Debug, Serialize, Deserialize)]
// pub struct StepDetails {
//     pub filename: Option<String>,
//     pub directory: Option<String>,
//     pub command: Option<String>,
//     pub package_name: Option<String>
// }


#[derive(Serialize, Deserialize, Debug)]
pub struct PairProgrammerStepRaw {
    pub step_number: String,
    pub heading: String,
    pub action: String,
    pub details: HashMap<String, String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StepChat{
    pub prompt: String,
    pub response: String
}


#[derive(Serialize, Deserialize, Debug)]
pub struct PairProgrammerStep {
    pub step_number: String,
    pub heading: String,
    pub action: String,
    pub details: HashMap<String, String>,
    pub response: String,
    pub executed: bool,
    pub chats: Vec<StepChat>,
}

impl PairProgrammerStep {
    // Constructor for StepData to simplify creation
    pub fn new(
        step_number: String,
        heading: String,
        action: String,
        details: HashMap<String, String>,
        response: String,
        executed: bool,
        chats: Vec<StepChat>,
   
    ) -> Self {
        Self {
            step_number,
            heading,
            action,
            details,
            response,
            executed,
            chats
        }
    }
}