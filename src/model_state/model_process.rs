use log::{debug, error};
use std::env;
use tokio::io::{ AsyncBufReadExt, BufReader };
use tokio::process::Command as tokio_command;
use std::collections::HashMap;
use super::state::ConfigSection;
use dirs::home_dir;
use std::fs::create_dir_all;
use std::fs;
use std::process::Command;
use crate::database::db_config::DB_INSTANCE;

fn get_system_ram_gb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap();
        let mem_total_line = meminfo
            .lines()
            .find(|line| line.starts_with("MemTotal"))
            .unwrap();
        let mem_kb: u64 = mem_total_line
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        mem_kb / 1024 / 1024 // Convert to GB
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .unwrap(); // This now works because output() is synchronous
        let mem_bytes: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap();
        mem_bytes / 1024 / 1024 / 1024 // Convert to GB
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        unimplemented!("Unsupported OS");
    }
}


//If the system's available RAM is 84 GB, the function select_config_section will iterate over the keys 
//in the provided config object to find the largest key that is less than or equal to the system's RAM.
//Here's a breakdown of the logic:
//It collects all keys from the config (in your case, "8", "16", "24", "32", "48", "64", "96") and 
//converts them to integers: 8, 16, 24, 32, 48, 64, 96.
//These keys are sorted in ascending order.
//The function iterates over the sorted keys, finding the largest one that is less than or equal to 84 GB.
//The largest key less than or equal to 84 is 64 (since 96 is greater than 84).
//Therefore, the selected_config will be the configuration with the key "64", 
fn select_config_section(config: &HashMap<String, ConfigSection>) -> &ConfigSection {
    let ram_gb = get_system_ram_gb();
    println!("System RAM: {} GB", ram_gb);

    // Convert keys to integers and sort them
    let mut ram_keys: Vec<u64> = config.keys()
        .map(|k| k.parse::<u64>().unwrap())
        .collect();
    ram_keys.sort();

    // Find the largest key less than or equal to ram_gb
    let mut selected_ram = ram_keys[0]; // default to the smallest config
    for &ram in ram_keys.iter() {
        if ram <= ram_gb {
            selected_ram = ram;
        } else {
            break;
        }
    }

    println!("Selected RAM configuration: {} GB", selected_ram);

    let selected_key = selected_ram.to_string();
    config.get(&selected_key).unwrap()
}



pub async fn kill_model_process(parent_pid: u32) -> Result<(), std::io::Error> {
    // Find the child process using pgrep
    let output = tokio_command::new("pgrep").arg("-P").arg(parent_pid.to_string()).output().await?;

    let child_pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if child_pid.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No child process found"));
    }

    // Kill the child process
    let kill_result = tokio_command::new("kill").arg("-9").arg(child_pid).output().await?;

    if !kill_result.status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to kill child process"));
    }

    Ok(())
}

// Function that starts the model process and takes a callback for the parent PID
pub async fn run_llama_server<F>(callback: F) where F: FnOnce(Option<u32>) + Send + 'static {
    // Retrieves the user's home directory or exits if it fails
    let home_dir = home_dir().expect("Failed to retrieve home directory");
    
    // Constructs the path to the `.pyano/configs` directory inside the home directory
    let config_dir = home_dir.join(".pyano/configs");    // Spawn a new thread for downloading the model and initialization
    
    // Ensures that the model directory exists by creating all directories in the path if they don't exist
    create_dir_all(&config_dir).expect("Failed to create config directory");

    // Joins the directory path with `model_config.json` to create the full path to the config file
    let config_file = config_dir.join("model_config.json");

    // Converts the `PathBuf` (config file path) into a string representation for further use
    let config_file_str = config_file
        .to_str()
        .expect("Failed to convert PathBuf to str")
        .to_string();

    // Attempts to read the content of the config file and handle potential errors
    let config_data = match fs::read_to_string(config_file_str){
        Ok(result) => result,
        Err(err) => {
            error!("Config model json cannot be loaded: {}", err);
            std::process::exit(1);
        }
    };

    // Parses the JSON content of the config file into a HashMap of String keys and ConfigSection values
    let config: HashMap<String, ConfigSection> = serde_json::from_str(&config_data)
    .expect("JSON was not well-formatted");

    // Selects the appropriate config section based on system RAM and available options in the config
    let selected_config = select_config_section(&config);


    // Retrieves the current working directory of the process
    let project_root = env::current_dir().unwrap();
    
    // Joins the current directory with the relative path to the script that will run the model
    //this is the shell script that containes the logic to run the model with llama-cpp and serves
    // the model on a http server
    let script_path = project_root.join("src/public/run-model.sh");

    // Spawns a new child process to run the shell script using `bash`, passing environment variables from the selected config
    let mut child = match
    tokio_command::new("bash")
            .arg(script_path)
            .env("MODEL_NAME", &selected_config.model_name)
            .env("MODEL_URL", &selected_config.model_url)
            .env("CTX_SIZE", selected_config.ctx_size.to_string())
            .env(
                "GPU_LAYERS_OFFLOADING",
                selected_config.gpu_layers_offloading.to_string(),
            )
            .env("BATCH_SIZE", selected_config.batch_size.to_string())
            .env(
                "MLOCK",
                selected_config.mlock.to_string(),
            )
            .env("MMAP", selected_config.mmap.to_string())
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
    {
        Ok(child) => {
            println!("Model process started successfully.");
            DB_INSTANCE.update_model_config(selected_config);
            child
        }
        Err(e) => {
            eprintln!("Failed to start model process: {}", e);
            return; // Exit if process can't be started
        }
    };

    debug!("Starting model");
    let pid = child.id();

    debug!("Model process ID: {}", pid.unwrap());
    callback(pid);

    // Capture and process the output in real-time
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap() {
        // Print the output from the shell script to the main thread stdout
        println!("Model server log: {}", line);
    }

    // Wait for the process to finish (don't await child directly, use .wait().await)
    // child.wait().await.expect("Failed to wait on llama.cpp server process");
    match child.wait().await {
        Ok(status) => {
            println!("Model process completed with status: {}", status);
        }
        Err(e) => {
            eprintln!("Failed to wait for the model process: {}", e);
        }
    }
}
