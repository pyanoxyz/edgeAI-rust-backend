use log::{ debug, error };
use dirs::home_dir;
// use psutil::host::info;
use tokio::io::{ AsyncBufReadExt, BufReader };
use tokio::process::Command as tokio_command;
// use std::collections::HashMap;
use super::state::AppConfigJson;
use std::fs::create_dir_all;
use std::fs;

// use std::process::Command;
use crate::database::db_config::DB_INSTANCE;

// fn get_system_ram_gb() -> u64 {
//     #[cfg(target_os = "linux")]
//     {
//         let meminfo = fs::read_to_string("/proc/meminfo").unwrap();
//         let mem_total_line = meminfo
//             .lines()
//             .find(|line| line.starts_with("MemTotal"))
//             .unwrap();
//         let mem_kb: u64 = mem_total_line.split_whitespace().nth(1).unwrap().parse().unwrap();
//         mem_kb / 1024 / 1024 // Convert to GB
//     }
//     #[cfg(target_os = "macos")]
//     {
//         let output = Command::new("sysctl").arg("-n").arg("hw.memsize").output().unwrap(); // This now works because output() is synchronous
//         let mem_bytes: u64 = String::from_utf8_lossy(&output.stdout).trim().parse().unwrap();
//         mem_bytes / 1024 / 1024 / 1024 // Convert to GB
//     }
//     #[cfg(not(any(target_os = "linux", target_os = "macos")))]
//     {
//         unimplemented!("Unsupported OS");
//     }
// }

pub fn get_app_config_json() -> String {
    let home_dir = home_dir().expect("Failed to retrieve home directory");

    // Constructs the path to the `.pyano/configs` directory inside the home directory
    let config_dir = home_dir.join(".pyano/configs"); // Spawn a new thread for downloading the model and initialization

    // TODO move this to dev scripts
    // Ensures that the model directory exists by creating all directories in the path if they don't exist
    create_dir_all(&config_dir).expect("Failed to create config directory");

    let config_file = config_dir.join("app-config.json");

    // Converts the `PathBuf` (config file path) into a string representation for further use
    let config_file_str = config_file
        .to_str()
        .expect("Failed to convert PathBuf to str")
        .to_string();

    // Attempts to read the content of the config file and handle potential errors
    let config_data = match fs::read_to_string(config_file_str) {
        Ok(result) => result,
        Err(err) => {
            error!("Config model json cannot be loaded: {}", err);
            std::process::exit(1);
        }
    };
    config_data
}

pub fn get_app_config() -> AppConfigJson {
    let config_data = get_app_config_json();

    let config: AppConfigJson = serde_json
        ::from_str(&config_data)
        .expect("JSON was not well-formatted");
    config
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
// modeltypes = base, solidity_typescript
pub async fn run_llama_server<F>(model_type: String, callback: F)
    where F: FnOnce(Option<u32>) + Send + 'static
{
    let config: AppConfigJson = get_app_config();

    let mut model_name = "";
    let mut ctx_size = &8192;
    let mut gpu_layers_offloading = &-1;
    let mut batch_size = &1024;
    let mut mlock = &false;
    let mut mmap = &false;
    let mut system_prompt: Option<&str> = None;

    if let Some(model) = config.get_model(&model_type) {
        model_name = &model.model_name;
        if let Some(config) = &model.model_config {
            ctx_size = &config.ctx_size;
            gpu_layers_offloading = &config.gpu_layers_offloading;
            batch_size = &config.batch_size;
            mlock = &config.mlock;
            mmap = &config.mmap;
            if config.system_prompt.chars().count() > 0 {
                system_prompt = Some(config.system_prompt.as_str());
            }
        }
    }
    let home_dir = home_dir().expect("Failed to retrieve home directory");
    let model_path = home_dir.join(".pyano/models").join(model_name);

    // check if model path exists
    debug!("Model path: {:#?}", model_path.to_str());
    if !model_path.exists() {
        error!("Model path does not exist: {:#?}", model_path.to_str());
        std::process::exit(1);
    }

    let scripts_dir = home_dir.join(".pyano/scripts");

    let script_path = scripts_dir.join("run-model.sh");
    // info!("Scripts path from where run_models.hs i sbeing loaded {:?}", script_path);

    // Spawns a new child process to run the shell script using `bash`, passing environment variables from the selected config
    let mut child = match
        tokio_command
            ::new("bash")
            .arg(script_path)
            .env("MODEL_NAME", model_name)
            .env("CTX_SIZE", ctx_size.to_string())
            .env("GPU_LAYERS_OFFLOADING", gpu_layers_offloading.to_string())
            .env("BATCH_SIZE", batch_size.to_string())
            .env("MLOCK", mlock.to_string())
            .env("MMAP", mmap.to_string())
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
    {
        Ok(child) => {
            println!("Model process started successfully.");
            if system_prompt.is_some() {
                DB_INSTANCE.update_system_prompt(model_name, system_prompt.unwrap());
            }
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
            println!("Model process completed with status: {} {}", status, model_type);
        }
        Err(e) => {
            eprintln!("Failed to wait for the model process: {}", e);
        }
    }
}
