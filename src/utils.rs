use std::env;
use std::process;
use sysinfo::{ System, SystemExt };

pub fn is_cloud_execution_mode() -> bool {
    load_env(); // Load the .env file from the specified path
    let cloud_mode = env::var("CLOUD_EXECUTION_MODE").unwrap_or_else(|_| "false".to_string());
    cloud_mode == "true"
}

pub fn get_local_url() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("LOCAL_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable LOCAL_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_remote_url() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("REMOTE_URL").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable REMOTE_URL is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_cloud_api_key() -> String {
    load_env(); // Load the .env file from the specified path
    env::var("CLOUD_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: Environment variable CLOUD_API_KEY is not set.");
        process::exit(1); // Exit the program with an error code
    })
}

pub fn get_llm_temperature() -> f64 {
    load_env(); // Load the .env file from the specified path
    env::var("TEMPERATURE")
        .unwrap_or_else(|_| {
            eprintln!("Error: Environment variable TEMPERATURE is not set.");
            process::exit(1); // Exit the program with an error code
        })
        .parse::<f64>()
        .unwrap_or_else(|_| {
            eprintln!("Error: Failed to parse TEMPERATURE as a float.");
            process::exit(1); // Exit with an error if parsing fails
        })
}
// Load the environment variables from a `.env` file
fn load_env() {
    let current_dir =  env::current_dir().unwrap();
    let top_dir = current_dir.parent().unwrap();
    let dotenv_path = top_dir.join(".env");
    dotenv::from_path(dotenv_path).ok();
}

pub fn get_total_ram() -> f64 {
    // Create a new System instance
    let mut system = System::new_all();

    // Refresh system information (e.g., RAM, CPU)
    system.refresh_memory();

    // Get total memory in kilobytes (KiB)
    let total_memory = system.total_memory();

    // Convert to megabytes (optional)
    let total_memory_gb = (total_memory as f64) / (1024.0 * 1024.0);
    total_memory_gb
}
