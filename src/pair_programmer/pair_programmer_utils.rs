
use serde_json::Value; // For handling JSON data
use actix_web:: Error;
use crate::database::db_config::DB_INSTANCE;
use super::types::{StepData, Step, StepsWrapper};


// pub fn parse_steps(input: &str) -> Vec<Step> {
//     // Regex for matching the step number and description
//     let re_step = Regex::new(r"(?i)\s*Step\s+(\d+)\s*:\s*(.+)").unwrap();
    
//     // Regex for matching the tool, allowing spaces and tool names with hyphens
//     let re_tool = Regex::new(r"(?i)\s*Tool\s*:\s*([\w\-]+)").unwrap();
    
//     // Regex for matching the action, including function name and parameters
//     let re_action = Regex::new(r#"(?i)\s*Action\s*:\s*<function=([^>]+)>\s*\{\{(.+?)\}\}\s*</function>"#).unwrap();
    
//     let mut steps = Vec::new();
//     let mut current_step_number = 0;
//     let mut current_heading = String::new();
//     let mut current_tool = String::new();
//     let mut current_action = String::new();

//     for line in input.lines() {
//         let trimmed_line = line.trim();
        
//         if let Some(caps) = re_step.captures(trimmed_line) {
//             // Push the previous step if exists
//             if current_step_number > 0 {
//                 steps.push(Step {
//                     step_number: current_step_number,
//                     heading: current_heading.clone(),
//                     tool: current_tool.clone(),
//                     action: current_action.clone(),
//                 });
//             }

//             // Start a new step
//             current_step_number = caps[1].parse().unwrap_or(0);
//             current_heading = caps[2].to_string();
//             current_tool.clear();
//             current_action.clear();
//         } else if let Some(caps) = re_tool.captures(trimmed_line) {
//             current_tool = caps[1].to_string();
//         } else if let Some(caps) = re_action.captures(trimmed_line) {
//             current_action = format!("<function={}>{{{{{}}}}}", &caps[1], &caps[2]);
//         }
//     }

//     // Add the last step if it exists
//     if current_step_number > 0 {
//         steps.push(Step {
//             step_number: current_step_number,
//             heading: current_heading.clone(),
//             tool: current_tool.clone(),
//             action: current_action.clone(),
//         });
//     }

//     steps
// }


pub fn parse_steps(json_data: &str) -> Result<Vec<Step>, Error> {
    let parsed: StepsWrapper = serde_json::from_str(json_data)?; // Parse into StepsWrapper first
    Ok(parsed.steps) // Extract the steps field and return
}



// Helper function to parse the step_number from a string to usize
pub fn parse_step_number(step_number_str: &str) -> Result<usize, Error> {
    step_number_str
        .parse::<usize>()
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid step number: unable to convert to a valid number"))
}

/// Validates whether a specified step can be executed in a sequence of steps.
///
/// This function performs several checks to ensure that the provided step can be executed:
/// 1. Ensures the step number is within the valid range of steps.
/// 2. Ensures that all previous steps have been executed before the specified step can be executed.
/// 3. Ensures the specified step has not already been executed.
///
/// # Arguments
///
/// * `step_number` - The 1-based index of the step to validate.
/// * `steps` - A vector of `serde_json::Value` representing the steps. Each step must be a JSON object
///             that contains an `"executed"` field, which indicates whether the step has already been executed.
///
/// # Returns
///
/// * `Ok(())` - If the specified step can be executed.
/// * `Err(Error)` - If validation fails due to one of the following reasons:
///     - The step number is out of bounds.
///     - A previous step has not been executed.
///     - The current step has already been executed.
///     - Invalid step format (the step data is not a JSON object).
///
/// # Errors
///
/// * `ErrorBadRequest` - Returned if:
///     - The step number is out of bounds.
///     - A previous step has not been executed.
///     - The current step has already been executed.
/// * `ErrorInternalServerError` - Returned if the step data is in an invalid format.
///
/// # Example
///
/// ```
/// let steps = vec![
///     json!({"executed": true}),
///     json!({"executed": false}),
///     json!({"executed": false}),
/// ];
///
/// let step_number = 2;
/// match validate_steps(step_number, &steps) {
///     Ok(()) => println!("Step can be executed."),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
///
/// In this example, the function will allow the second step to be executed only if the first step has already been executed
/// and the second step itself has not been executed yet.

pub fn validate_steps(step_number: usize, steps: &Vec<serde_json::Value>) -> Result<(), Error> {

    if step_number > steps.len() {
        return Err(actix_web::error::ErrorBadRequest(
            format!("Step number {} is out of bounds, there are only {} steps", step_number, steps.len()),
        ));
    }

    for (index, step) in steps.into_iter().enumerate() {
        let actual_index = index + 1; // Start enumeration from 1

        // Access step data as an object
        let step_data = step.as_object().ok_or_else(|| {
            actix_web::error::ErrorInternalServerError("Invalid step data format")
        })?;

        // Check if the current step is the one we want to execute
        if actual_index == step_number {
            let executed = step_data.get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // If the step is already executed, return an error
            if executed {
                return Err(actix_web::error::ErrorBadRequest(
                    format!("Step {} has already been executed", step_number),
                ));
            }
        }

        // Ensure that all previous steps are executed
        // if actual_index < step_number {
        //     let previous_executed = step_data.get("executed")
        //         .and_then(|v| v.as_bool())
        //         .unwrap_or(false);

        //     if !previous_executed {
        //         return Err(actix_web::error::ErrorBadRequest(
        //             format!("Previous step {} has not been executed", actual_index),
        //         ));
        //     }
        // }
    }
    Ok(())
}


pub fn format_steps(steps: &[Value], step_number: usize) -> (String, String) {
    // Format all steps
    let all_steps = steps.iter()
        .enumerate()
        .map(|(index, step)| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            format!("Step: {}. {}", index + 1, heading)
        })
        .collect::<Vec<String>>()
        .join("\n");


    // Format steps executed with response (output all steps before the current step_number)
    let steps_executed_with_response = steps.iter()
        .take(step_number)  // Take all steps up to the current step_number
        .filter(|step| {
            step.get("tool").and_then(|v| v.as_str()) == Some("edit_file") &&
            step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false)
        })
        .map(|step| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            let response = step.get("response").and_then(|v| v.as_str()).unwrap_or("No Response");
            format!("Step: {}\n response: {}\n", heading, response)
        })
        .collect::<Vec<String>>()
        .join("\n");


    (all_steps, steps_executed_with_response)
}

pub fn prompt_with_context(
    all_steps: &str, 
    steps_executed: &str, 
    current_step: &str, 
    additional_context_from_codebase: &str, 
    recent_discussion: &str
) -> String {
    format!(
        r#"
        all_steps: {all_steps}
        steps_executed_so_far: {steps_executed}
        current_step: {current_step}
        overall_context: {additional_context_from_codebase}
        recent_discussion: {recent_discussion}
        Please implement the current step based on this context. Ensure your response follows the specified output format in the system prompt.
        "#,
        all_steps = all_steps,
        steps_executed = steps_executed,
        current_step = current_step,
        additional_context_from_codebase = additional_context_from_codebase,
        recent_discussion = recent_discussion
    )
}

pub fn prompt_with_context_for_chat(
    all_steps: &str, 
    steps_executed: &str, 
    current_step: &str, 
    user_prompt: &str, 
    recent_discussion: &str
) -> String {
    format!(
        r#"
        all_steps: {all_steps}
        steps_executed_so_far: {steps_executed}
        current_step: {current_step}
        recent_discussion: {recent_discussion}
        Please respond to user query {user_prompt} based on the context.
        "#,
        all_steps = all_steps,
        steps_executed = steps_executed,
        current_step = current_step,
        user_prompt = user_prompt,
        recent_discussion = recent_discussion
    )
}

pub fn rethink_prompt_with_context(
    all_steps: &str, 
    steps_executed: &str, 
    current_step: &str, 
    recent_discussion: &str
) -> String {
    format!(
        r#"
        all_steps: {all_steps}
        steps_executed_so_far: {steps_executed}
        current_step: {current_step}
        recent_discussion: {recent_discussion}
        Please suggest changes to the current step based on the recent discussion.
        "#,
        all_steps = all_steps,
        steps_executed = steps_executed,
        current_step = current_step,
        recent_discussion = recent_discussion
    )
}


pub fn data_validation(pair_programmer_id: &str, step_number: &str) -> Result<StepData, Error> {
    let step_number = parse_step_number(step_number).map_err(|err| {
        actix_web::error::ErrorBadRequest(format!("Invalid step number: {}", err))
    })?;
    let true_step_number = step_number - 1;

    let steps = DB_INSTANCE.fetch_steps(&pair_programmer_id);

    // Validate steps
    validate_steps(step_number, &steps).map_err(|err| {
        actix_web::error::ErrorBadRequest(format!("Step validation failed: {}", err))
    })?;



    // Fetch the current step based on true_step_number
    let step = steps.get(true_step_number).ok_or_else(|| {
        actix_web::error::ErrorBadRequest(format!("Step number out of range: {}", true_step_number))
    })?;


    let tool = step
        .get("tool")
        .and_then(|v| v.as_str())
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Tool field is missing or not a string"))?;

    if tool != "edit_file" {
        return Err(actix_web::error::ErrorBadRequest("Tool must be 'edit_file'").into());
    }

    let step_chat = DB_INSTANCE
        .step_chat_string(pair_programmer_id, &step_number.to_string())
        .map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("Failed to retrieve chat: {:?}", err))
        })?;

    // Retrieve the task heading from the step
    let task_heading = step
        .get("heading")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest(format!(
                "Invalid step: 'heading' field is missing or not a string for step {}",
                true_step_number
            ))
        })?;

    let function_call = "";

    let (all_steps, steps_executed_response) =
        format_steps(&steps, step_number);

        Ok(StepData {
            step_number,
            task_heading: task_heading.to_owned(),
            function_call: function_call.to_string(),
            step_chat,
            all_steps,
            steps_executed_response
        })
        
}
