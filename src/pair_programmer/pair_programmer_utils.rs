
use actix_web:: Error;
use crate::database::db_config::DB_INSTANCE;
use super::types::PairProgrammerStepRaw;
use regex::Regex;
use std::collections::HashMap;


fn parse_heading(input_text: &str) -> Option<String> {
    let pattern = r"\bheading:\s*(.*?)(?:\s*\n|\s*\baction\b)";
    let re = Regex::new(pattern).unwrap();
    if let Some(caps) = re.captures(input_text) {
        return Some(caps.get(1).unwrap().as_str().trim().to_string());
    }
    None
}

fn parse_action(input_text: &str) -> Option<String> {
    let pattern = r"\baction:\s*(.*?)(?:\s*\n|\s*\bdetails\b)";
    let re = Regex::new(pattern).unwrap();
    if let Some(caps) = re.captures(input_text) {
        return Some(caps.get(1).unwrap().as_str().trim().to_string());
    }
    None
}

fn parse_details(input_text: &str) -> Option<String> {
    let pattern = r"\bdetails:\s*(.*)";
    let re = Regex::new(pattern).unwrap();
    if let Some(caps) = re.captures(input_text) {
        return Some(caps.get(1).unwrap().as_str().trim().to_string());
    }
    None
}

fn parse_key_values(input_text: Option<&str>) -> Option<std::collections::HashMap<String, String>> {
    if input_text.is_none() {
        return None;
    }
    let input_text = input_text.unwrap();
    let mut key_value_pairs = std::collections::HashMap::new();
    for line in input_text.lines() {
        if let Some((key, value)) = line.split_once(":") {
            key_value_pairs.insert(key.trim().to_string(), value.trim().trim_matches('"').to_string());
        }
    }
    Some(key_value_pairs)
}
pub fn parse_steps(input: &str) -> Result<Vec<PairProgrammerStepRaw>, Error> {
    let split_input = input.split("step_number").skip(1); // Split based on "step_number" and skip the first empty segment
    let mut steps = Vec::new();

    for (index, step_text) in split_input.enumerate() {
        let step_number = (index + 1).to_string();

        let heading = match parse_heading(step_text) {
            Some(h) => h,
            None => String::from("Missing heading"),
        };

        let action = match parse_action(step_text) {
            Some(a) => a,
            None => String::from("Missing action"),
        };

        let details_str = parse_details(step_text);
        let details = match parse_key_values(details_str.as_deref()) {
            Some(d) => d,
            None => HashMap::new(),
        };

        steps.push(PairProgrammerStepRaw {
            step_number,
            heading,
            action,
            details,
        });
    }

    Ok(steps)
}


// Helper function to parse the step_number from a string to usize
pub fn parse_step_number(step_number_str: &str) -> Result<usize, Error> {
    step_number_str
        .parse::<usize>()
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid step number: unable to convert to a valid number"))
}



// pub fn validate_steps(step_number: usize, steps: &Vec<serde_json::Value>) -> Result<(), Error> {

//     if step_number > steps.len() {
//         return Err(actix_web::error::ErrorBadRequest(
//             format!("Step number {} is out of bounds, there are only {} steps", step_number, steps.len()),
//         ));
//     }

//     for (index, step) in steps.into_iter().enumerate() {
//         let actual_index = index + 1; // Start enumeration from 1

//         // Access step data as an object
//         let step_data = step.as_object().ok_or_else(|| {
//             actix_web::error::ErrorInternalServerError("Invalid step data format")
//         })?;

//         // Check if the current step is the one we want to execute
//         if actual_index == step_number {
//             let executed = step_data.get("executed")
//                 .and_then(|v| v.as_bool())
//                 .unwrap_or(false);

//             // If the step is already executed, return an error
//             if executed {
//                 return Err(actix_web::error::ErrorBadRequest(
//                     format!("Step {} has already been executed", step_number),
//                 ));
//             }
//         }

//         // Ensure that all previous steps are executed
//         // if actual_index < step_number {
//         //     let previous_executed = step_data.get("executed")
//         //         .and_then(|v| v.as_bool())
//         //         .unwrap_or(false);

//         //     if !previous_executed {
//         //         return Err(actix_web::error::ErrorBadRequest(
//         //             format!("Previous step {} has not been executed", actual_index),
//         //         ));
//         //     }
//         // }
//     }
//     Ok(())
// }


// pub fn format_steps(steps: &[Value], step_number: usize) -> (String, String) {
//     // Format all steps
//     let all_steps = steps.iter()
//         .enumerate()
//         .map(|(index, step)| {
//             let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
//             format!("Step: {}. {}", index + 1, heading)
//         })
//         .collect::<Vec<String>>()
//         .join("\n");


//     // Format steps executed with response (output all steps before the current step_number)
//     let steps_executed_with_response = steps.iter()
//         .take(step_number)  // Take all steps up to the current step_number
//         .filter(|step| {
//             step.get("tool").and_then(|v| v.as_str()) == Some("edit_file") &&
//             step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false)
//         })
//         .map(|step| {
//             let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
//             let response = step.get("response").and_then(|v| v.as_str()).unwrap_or("No Response");
//             format!("Step: {}\n response: {}\n", heading, response)
//         })
//         .collect::<Vec<String>>()
//         .join("\n");


//     (all_steps, steps_executed_with_response)
// }

pub fn prompt_with_context(
    pair_programmer_id: &str,
    all_steps: &str, 
    steps_executed: &str, 
    current_step: &str, 
    additional_context_from_codebase: &str, 
) -> String {

    let original_task = DB_INSTANCE.fetch_task_from_pair_programmer(&pair_programmer_id).unwrap();
    format!(
        r#"
        original_task: {original_task}
        all_steps: {all_steps}
        executed_steps: {steps_executed}
        current_step: {current_step}
        overall_context: {additional_context_from_codebase}
        Please implement the current step based on this overall_context. Ensure your response follows the specified output format in the system prompt.
        "#,
        all_steps = all_steps,
        steps_executed = steps_executed,
        current_step = current_step,
        additional_context_from_codebase = additional_context_from_codebase,
    )
}

// pub fn prompt_with_context_for_chat(
//     all_steps: &str, 
//     steps_executed: &str, 
//     current_step: &str, 
//     user_prompt: &str, 
//     recent_discussion: &str
// ) -> String {
//     format!(
//         r#"
//         all_steps: {all_steps}
//         steps_executed_so_far: {steps_executed}
//         current_step: {current_step}
//         recent_discussion: {recent_discussion}
//         Please respond to user query {user_prompt} based on the context.
//         "#,
//         all_steps = all_steps,
//         steps_executed = steps_executed,
//         current_step = current_step,
//         user_prompt = user_prompt,
//         recent_discussion = recent_discussion
//     )
// }

// pub fn rethink_prompt_with_context(
//     all_steps: &str, 
//     steps_executed: &str, 
//     current_step: &str, 
//     recent_discussion: &str
// ) -> String {
//     format!(
//         r#"
//         all_steps: {all_steps}
//         steps_executed_so_far: {steps_executed}
//         current_step: {current_step}
//         recent_discussion: {recent_discussion}
//         Please suggest changes to the current step based on the recent discussion.
//         "#,
//         all_steps = all_steps,
//         steps_executed = steps_executed,
//         current_step = current_step,
//         recent_discussion = recent_discussion
//     )
// }


// pub fn data_validation(pair_programmer_id: &str, step_number: &str) -> Result<StepData, Error> {
//     let step_number = parse_step_number(step_number).map_err(|err| {
//         actix_web::error::ErrorBadRequest(format!("Invalid step number: {}", err))
//     })?;
//     let true_step_number = step_number - 1;

//     let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);

//     // Validate steps
//     validate_steps(step_number, &steps).map_err(|err| {
//         actix_web::error::ErrorBadRequest(format!("Step validation failed: {}", err))
//     })?;



//     // Fetch the current step based on true_step_number
//     let step = steps.get(true_step_number).ok_or_else(|| {
//         actix_web::error::ErrorBadRequest(format!("Step number out of range: {}", true_step_number))
//     })?;


//     let tool = step
//         .get("action")
//         .and_then(|v| v.as_str())
//         .ok_or_else(|| actix_web::error::ErrorBadRequest("Action field is missing or not a string"))?;

//     // if tool != "edit_file" {
//     //     return Err(actix_web::error::ErrorBadRequest("Tool must be 'edit_file'").into());
//     // }

//     // let step_chat = DB_INSTANCE
//     //     .step_chat_string(pair_programmer_id, &step_number.to_string())
//     //     .map_err(|err| {
//     //         actix_web::error::ErrorInternalServerError(format!("Failed to retrieve chat: {:?}", err))
//     //     })?;

//     // Retrieve the task heading from the step
//     let task_heading = step
//         .get("heading")
//         .and_then(|v| v.as_str())
//         .ok_or_else(|| {
//             actix_web::error::ErrorBadRequest(format!(
//                 "Invalid step: 'heading' field is missing or not a string for step {}",
//                 true_step_number
//             ))
//         })?;

//     let function_call = "";

//     let (all_steps, steps_executed_response) =
//         format_steps(&steps, step_number);

//         Ok(StepData {
//             step_number,
//             task_heading: task_heading.to_owned(),
//             function_call: function_call.to_string(),
//             step_chat,
//             all_steps,
//             steps_executed_response
//         })
        
// }