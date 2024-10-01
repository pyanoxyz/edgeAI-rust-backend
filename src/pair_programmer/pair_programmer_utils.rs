
use regex::Regex;
use serde_json::Value; // For handling JSON data
use crate::pair_programmer::pair_programmer_types::Step;
use std::cmp::max;
use actix_web:: Error;

/// Parses a string input and extracts steps with their associated metadata, including step number, heading, tool, and action.
///
/// This function uses regular expressions to parse each line of the input string, matching the following patterns:
/// 1. **Step**: Captures the step number and heading in the format `Step N: Description`.
/// 2. **Tool**: Captures the tool name, allowing for spaces and hyphens, in the format `Tool: tool-name`.
/// 3. **Action**: Captures an action that contains a function and parameters in the format 
///    `Action: <function=function_name>{{parameters}}`
///
/// Each step is represented by a `Step` struct that holds the following:
/// - `step_number`: The step's number.
/// - `heading`: A string representing the heading or description of the step.
/// - `tool`: The tool name associated with the step, if provided.
/// - `action`: The action to be executed, containing a function and its parameters, if provided.
///
/// # Arguments
///
/// * `input` - A string slice (`&str`) containing the step definitions. The steps should be formatted in a predefined structure with step number, tool, and action.
///
/// # Returns
///
/// * `Vec<Step>` - A vector of `Step` structs, each representing a parsed step with its metadata.
///
/// # Example
///
/// ```rust
/// let input = r#"
/// Step 1: Initialize the project
/// Tool: build-tool
/// Action: <function=initialize>{{"param": "value"}}
///
/// Step 2: Set up environment
/// Tool: env-tool
/// Action: <function=setup>{{"config": "env"}}
/// "#;
///
/// let steps = parse_steps(input);
/// for step in steps {
///     println!("Step {}: {}", step.step_number, step.heading);
///     println!("Tool: {}", step.tool);
///     println!("Action: {}", step.action);
/// }
/// ```
///
/// This example demonstrates how the function processes the input and extracts the steps, tools, and actions. 
/// The output will be a list of `Step` structs, with each step containing its parsed attributes.
///
/// # Note
/// - If a step does not contain a tool or an action, the corresponding fields will remain empty.
/// - The last step is automatically added when the end of the input is reached.

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

/// Formats the steps in a structured way, generating three different outputs:
/// 1. A formatted list of all steps with their headings.
/// 2. A formatted list of steps that have been executed so far.
/// 3. A formatted list of the most recent executed steps (up to the last 3 before the current step),
///    along with their associated responses.
///
/// # Arguments
///
/// * `steps` - A slice of `serde_json::Value` representing the steps. Each step is expected to contain:
///   - `"heading"`: A string representing the heading of the step.
///   - `"executed"`: A boolean indicating whether the step has been executed.
///   - `"response"`: A string representing the response associated with the step, if available.
/// * `step_number` - The current step number, used to filter and limit the steps executed with responses.
///
/// # Returns
///
/// A tuple containing three formatted strings:
/// * `(String, String, String)`:
///   - The first string contains all steps with their headings in the format `Step: N. Heading`.
///   - The second string contains the steps that have been executed so far, filtered by the `"executed"` field.
///   - The third string contains the last 3 executed steps (limited to steps before the current step number),
///     including their responses, in the format:
///     ```
///     Step: Heading
///     response: Response
///     ```
///
/// # Example
///
/// ```rust
/// let steps = vec![
///     json!({"heading": "Initialize project", "executed": true, "response": "Success"}),
///     json!({"heading": "Set up environment", "executed": false}),
///     json!({"heading": "Run tests", "executed": true, "response": "All tests passed"}),
/// ];
///
/// let (all_steps, steps_executed_so_far, steps_executed_with_response) = format_steps(&steps, 3);
///
/// println!("All Steps:\n{}", all_steps);
/// println!("Executed Steps So Far:\n{}", steps_executed_so_far);
/// println!("Executed Steps with Responses:\n{}", steps_executed_with_response);
/// ```
///
/// This example shows how to use the function to get the formatted steps and how the output is structured.

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