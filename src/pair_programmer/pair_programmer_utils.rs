
use regex::Regex;
use crate::pair_programmer::pair_programmer_types::Step;


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
