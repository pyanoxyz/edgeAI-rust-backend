use log::debug;
use std::env;
use tokio::io::{ AsyncBufReadExt, BufReader };
use tokio::process::Command;

pub async fn kill_model_process(parent_pid: u32) -> Result<(), std::io::Error> {
    // Find the child process using pgrep
    let output = Command::new("pgrep").arg("-P").arg(parent_pid.to_string()).output().await?;

    let child_pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if child_pid.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No child process found"));
    }

    // Kill the child process
    let kill_result = Command::new("kill").arg("-9").arg(child_pid).output().await?;

    if !kill_result.status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to kill child process"));
    }

    Ok(())
}

// Function that starts the model process and takes a callback for the parent PID
pub async fn run_llama_server<F>(callback: F) where F: FnOnce(Option<u32>) + Send + 'static {
    // Make sure the callback is FnOnce, accepts Option<u32>, and is Send
    let project_root = env::current_dir().unwrap();
    let script_path = project_root.join("src/public/run-model.sh");

    let mut child = match
        Command::new("sh")
            .arg(script_path) // Path to your llama.cpp script
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
    {
        Ok(child) => {
            println!("Model process started successfully.");
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
