
use regex::Regex;
use serde_json::Value; // For handling JSON data
use crate::pair_programmer::pair_programmer_types::Step;
use std::cmp::max;
use actix_web:: Error;

pub fn parse_steps(input: &str) -> Vec<Step> {
    // Regex for matching the step number and description
    let re_step = Regex::new(r"(?i)\s*Step\s+(\d+)\s*:\s*(.+)").unwrap();
    
    // Regex for matching the tool, allowing spaces and tool names with hyphens
    let re_tool = Regex::new(r"(?i)\s*Tool\s*:\s*([\w\-]+)").unwrap();
    
    // Regex for matching the action, including function name and parameters
    let re_action = Regex::new(r#"(?i)\s*Action\s*:\s*<function=([^>]+)>\s*\{\{(.+?)\}\}\s*</function>"#).unwrap();
    
    let mut steps = Vec::new();
    let mut current_step_number = 0;
    let mut current_heading = String::new();
    let mut current_tool = String::new();
    let mut current_action = String::new();

    for line in input.lines() {
        let trimmed_line = line.trim();
        
        if let Some(caps) = re_step.captures(trimmed_line) {
            // Push the previous step if exists
            if current_step_number > 0 {
                steps.push(Step {
                    step_number: current_step_number,
                    heading: current_heading.clone(),
                    tool: current_tool.clone(),
                    action: current_action.clone(),
                });
            }

            // Start a new step
            current_step_number = caps[1].parse().unwrap_or(0);
            current_heading = caps[2].to_string();
            current_tool.clear();
            current_action.clear();
        } else if let Some(caps) = re_tool.captures(trimmed_line) {
            current_tool = caps[1].to_string();
        } else if let Some(caps) = re_action.captures(trimmed_line) {
            current_action = format!("<function={}>{{{{{}}}}}", &caps[1], &caps[2]);
        }
    }

    // Add the last step if it exists
    if current_step_number > 0 {
        steps.push(Step {
            step_number: current_step_number,
            heading: current_heading.clone(),
            tool: current_tool.clone(),
            action: current_action.clone(),
        });
    }

    steps
}

// Helper function to parse the step_number from a string to usize
pub fn parse_step_number(step_number_str: &str) -> Result<usize, Error> {
    step_number_str
        .parse::<usize>()
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid step number: unable to convert to a valid number"))
}

// Helper function to validate whether the steps can be executed
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
        if actual_index < step_number {
            let previous_executed = step_data.get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !previous_executed {
                return Err(actix_web::error::ErrorBadRequest(
                    format!("Previous step {} has not been executed", actual_index),
                ));
            }
        }
    }
    Ok(())
}

pub fn format_steps(steps: &[Value], step_number: usize) -> (String, String, String) {
    // Format all steps
    let all_steps = steps.iter()
        .enumerate()
        .map(|(index, step)| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            format!("Step: {}. {}", index + 1, heading)
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Format steps executed so far
    let steps_executed_so_far = steps.iter()
        .enumerate()
        .filter(|(_, step)| step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|(index, step)| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            format!("Step: {}. {}", index + 1, heading)
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Format steps executed with response (limit to last 3 before current step_number)
    let steps_executed_with_response = steps.iter()
        .skip(max(0, step_number.saturating_sub(3)))  // Start from max(0, step_number-3)
        .take(step_number)  // Take up to current step_number
        .filter(|step| step.get("executed").and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|step| {
            let heading = step.get("heading").and_then(|v| v.as_str()).unwrap_or("No Heading");
            let response = step.get("response").and_then(|v| v.as_str()).unwrap_or("No Response");
            format!("Step: {}\n response: {}\n", heading, response)
        })
        .collect::<Vec<String>>()
        .join("\n");

    (all_steps, steps_executed_so_far, steps_executed_with_response)
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