use std::env;

pub fn is_cloud_execution_mode() -> bool {
    // load_env(); // Load the .env file from the specified path
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}

pub fn get_local_url() -> String {
    // load_env(); // Load the .env file from the specified path
    env::var("LOCAL_URL").unwrap_or_else(|_| {
        eprintln!("Warning: Environment variable LOCAL_URL is not set. Using default value [http://localhost:52555]");
        "http://localhost:52555".to_string() // Default value for LOCAL_URL
    })
}

pub fn get_infill_local_url() -> String {
    // load_env(); // Load the .env file from the specified path
    env::var("INFILL_LOCAL_URL").unwrap_or_else(|_| {
        eprintln!("Warning: Environment variable INFILL_LOCAL_URL is not set. Using default value [http://localhost:52554]");
        "http://localhost:52554".to_string() // Default value for LOCAL_URL
    })
}


pub fn get_remote_url() -> String {
    // load_env(); // Load the .env file from the specified path
    env::var("REMOTE_URL").unwrap_or_else(|_| {
        eprintln!("Warning: Environment variable LOCAL_URL is not set. Using default value.");
        "http://localhost:8000".to_string() // Default value for LOCAL_URL
    })
}


pub fn get_cloud_api_key() -> String {
    // load_env(); // Load the .env file from the specified path
    env::var("CLOUD_API_KEY").unwrap_or_else(|_| {
        eprintln!("Warning: Environment variable LOCAL_URL is not set. Using default value.");
        "none".to_string() // Default value for LOCAL_URL
    })
}


pub fn get_llm_temperature() -> f64 {
    // load_env(); // Load the .env file from the specified path
    env::var("TEMPERATURE")
        .unwrap_or_else(|_| {
            eprintln!("Warning: Environment variable TEMPERATURE is not set. Using default value of 0.4.");
            "0.4".to_string() // Use default value "0.4" as a string
        })
        .parse::<f64>()
        .unwrap_or_else(|_| {
            eprintln!("Error: Failed to parse TEMPERATURE as a float. Using default value of 0.4.");
            0.4 // Default value if parsing fails
        })
}
