use log::debug;
use dirs::home_dir;
use tokio::io::{ AsyncBufReadExt, BufReader };
use tokio::process::Command as tokio_command;
use tokio::task::JoinHandle;
use std::fs::create_dir_all;

// Function that starts the model process and takes a callback for the parent PID
pub async fn run_infill_server<F>(callback: F, reset_state_callback: impl Fn() -> JoinHandle<()>)
    where F: FnOnce(Option<u32>) + Send + 'static
{
    // Retrieves the user's home directory or exits if it fails
    let home_dir = home_dir().expect("Failed to retrieve home directory");

    // Constructs the path to the `.pyano/configs` directory inside the home directory
    let config_dir = home_dir.join(".pyano/configs"); // Spawn a new thread for downloading the model and initialization

    // Ensures that the model directory exists by creating all directories in the path if they don't exist
    create_dir_all(&config_dir).expect("Failed to create config directory");

    // Retrieves the current working directory of the process
    let scripts_dir = home_dir.join(".pyano/scripts");

    let script_path = scripts_dir.join("run-infill-model.sh");

    // Spawns a new child process to run the shell script using `bash`, passing environment variables from the selected config
    let mut child = match
        tokio_command
            ::new("bash")
            .arg(script_path)
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
    {
        Ok(child) => {
            println!("Infill Model process started successfully.");

            child
        }
        Err(e) => {
            eprintln!("Failed to start Infill model process: {}", e);
            reset_state_callback();
            return; // Exit if process can't be started
        }
    };

    debug!("Starting infill model");
    let pid = child.id();

    debug!("Infill Model process ID: {}", pid.unwrap());
    callback(pid);

    // Capture and process the output in real-time
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap() {}

    // Wait for the process to finish (don't await child directly, use .wait().await)
    // child.wait().await.expect("Failed to wait on llama.cpp server process");
    match child.wait().await {
        Ok(status) => {
            reset_state_callback();
            println!("Infill Model process completed with status: {}", status);
        }
        Err(e) => {
            reset_state_callback();
            eprintln!("Failed to wait for the  Infill model process: {}", e);
        }
    }
}
