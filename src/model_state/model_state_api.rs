use crate::ModelState;
use actix_web::{ get, web, HttpResponse, Error };
use serde_json::json;
use log::debug;
use std::sync::Arc;
use crate::model_state::model_process::{ kill_model_process, run_llama_server };
use sysinfo::{ ProcessExt, System, SystemExt };
use serde::Deserialize;
use tokio::time::{ sleep, Duration, Instant }; // Import sleep and Duration from tokio
use tokio::process::Command;

pub fn model_state_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(mode_state)
        .service(run_model)
        .service(kill_model)
        .service(restart_model)
        .service(get_model_usage);
}

#[get("/model-state")]
pub async fn mode_state(
    data: web::Data<Arc<ModelState>>
) -> Result<HttpResponse, Error> {
    let model_process_guard = data.model_process.lock().await;
    let model_pid_guard = data.model_pid.lock().unwrap();

    let model_running = model_process_guard.is_some(); // Check if model process exists
    let pid = *model_pid_guard;

    // Debug logging to show process state and PID
    if let Some(pid) = pid {
        debug!("MODEL PID exists: {}", pid);
    } else {
        debug!("No PID found");
    }

    if model_running {
        Ok(
            HttpResponse::Ok().json(
                json!({
                "message": "Model is already running", "running": true, "pid": pid.unwrap_or(0)
            })
            )
        )
    } else {
        Ok(
            HttpResponse::BadRequest().json(
                json!({
                "message": "Model not started", "running": false
            })
            )
        )
    }
}

#[get("/run-model")]
async fn run_model(
    data: web::Data<Arc<ModelState>>
) -> Result<HttpResponse, Error> {
    let mut model_process_guard = data.model_process.lock().await;
    let model_pid_guard = data.model_pid.lock().unwrap();

    // Check if model is already running
    let model_running = model_process_guard.is_some();
    if model_running {
        return Ok(
            HttpResponse::BadRequest().json(
                json!({"message": "Model is already running", "pid": model_pid_guard.unwrap_or(0)})
            )
        );
    }

    // Define a callback that stores the process PID in the shared state
    let callback = {
        let data_clone = data.clone(); // Clone the Arc<ModelState> to extend the lifetime
        move |pid: Option<u32>| {
            let mut model_pid_guard = data_clone.model_pid.lock().unwrap();
            *model_pid_guard = pid; // Store the PID in shared state
        }
    };

    // Start the llama process and get the PID using the callback
    let handle = tokio::spawn(async { run_llama_server(callback).await });

    *model_process_guard = Some(handle);

    // Log that the script has been triggered
    println!("Main server thread running...");

    Ok(HttpResponse::Ok().json(json!({"message": "Model started"})))
}

#[get("/kill-model")]
async fn kill_model(
    data: web::Data<Arc<ModelState>>
) -> Result<HttpResponse, Error> {
    let mut model_process_guard = data.model_process.lock().await;
    let mut model_pid_guard = data.model_pid.lock().unwrap();

    // Early return if model is not running
    let parent_pid = match *model_pid_guard {
        Some(pid) => pid,
        None => {
            return Ok(
                HttpResponse::BadRequest().json(
                    json!({"message": "No running model found"})
                )
            );
        }
    };
    // Kill the child process
    if let Err(e) = kill_model_process(parent_pid).await {
        return Ok(
            HttpResponse::InternalServerError().json(
                json!({"message": format!("Failed to kill model process: {}", e)})
            )
        );
    }

    *model_pid_guard = None; // Reset the PID after killing the child process
    *model_process_guard = None;
    return Ok(
        HttpResponse::Ok().json(
            json!({"message": "Model stopped successfully"})
        )
    );
}

#[get("/restart-model")]
async fn restart_model(
    data: web::Data<Arc<ModelState>>
) -> Result<HttpResponse, Error> {
    // Kill the model if it's running

    let mut model_process_guard = data.model_process.lock().await;
    let mut model_pid_guard = data.model_pid.lock().unwrap();

    let _parent_pid = match *model_pid_guard {
        Some(pid) => pid,
        None => {
            return Ok(
                HttpResponse::BadRequest().json(
                    json!({"message": "No running model found"})
                )
            );
        }
    };

    if let Some(pid) = *model_pid_guard {
        if let Err(e) = kill_model_process(pid).await {
            return Ok(
                HttpResponse::InternalServerError().json(
                    json!({"message": format!("Failed to kill the model process: {}", e)})
                )
            );
        }
    }
    // Reset the state and start a new model
    *model_pid_guard = None;
    *model_process_guard = None;

    // TODO needs refactoring
    // Call the run_model logic to start the new model
    // Define a callback that stores the process PID in the shared state
    let callback = {
        let data_clone = data.clone(); // Clone the Arc<ModelState> to extend the lifetime
        move |pid: Option<u32>| {
            let mut model_pid_guard = data_clone.model_pid.lock().unwrap();
            *model_pid_guard = pid; // Store the PID in shared state
        }
    };

    // Start the llama process and get the PID using the callback
    let handle = tokio::spawn(async { run_llama_server(callback).await });

    *model_process_guard = Some(handle);

    // Log that the script has been triggered
    println!("Main server thread running...");

    Ok(HttpResponse::Ok().json(json!({"message": "Model started"})))
}

#[derive(Deserialize)]
struct GetModelUsageParams {
    interval: Option<f64>, // Use Option<f64> to allow it to be optional
}

#[get("/get-model-usage")]
async fn get_model_usage(
    data: web::Data<Arc<ModelState>>,
    query: web::Query<GetModelUsageParams>
) -> Result<HttpResponse, Error> {
    let interval = query.interval.unwrap_or(0.5);
    let model_pid_guard = data.model_pid.lock().unwrap();

    // Get the model PID
    let pid = match *model_pid_guard {
        Some(pid) => pid,
        None => {
            return Ok(
                HttpResponse::BadRequest().json(
                    json!({"message": "No running model found"})
                )
            );
        }
    };

    // Create a System instance to access process information
    let mut system = System::new_all();

    // Refresh the system information
    system.refresh_all();
    let output = Command::new("pgrep")
        .arg("-P")
        .arg(pid.to_string())
        .output().await?;

    let child_pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if child_pid.is_empty() {
        return Ok(
            HttpResponse::InternalServerError().json(
                json!({"message": "Failed to get process information"})
            )
        );
    }

    // Find the process by PID
    for (process_pid, process) in system.processes() {
        if process_pid.to_string() == child_pid {
            let mut system_for_refresh = sysinfo::System::new_all();
            let cpu_usage = calculate_cpu_usage(
                &mut system_for_refresh,
                process,
                interval
            ).await;
            let ram_usage_mb = (process.memory() as f64) / (1024.0 * 1024.0); // Convert bytes to MB

            return Ok(
                HttpResponse::Ok().json(
                    json!({ "pid": pid,"cpu_percentage": cpu_usage, "ram_megabytes": ram_usage_mb
            })
                )
            );
        }
    }

    Ok(
        HttpResponse::InternalServerError().json(
            json!({"message": "Failed to get process information"})
        )
    )
}

// Function to calculate CPU usage based on the interval
async fn calculate_cpu_usage(
    system: &mut sysinfo::System,
    process: &sysinfo::Process,
    interval: f64
) -> f32 {
    // Get initial CPU usage
    let initial_cpu = process.cpu_usage();
    let initial_time = Instant::now();

    // Wait for the specified interval
    sleep(Duration::from_secs_f64(interval)).await;

    system.refresh_process(process.pid());

    // Refresh the system info and get the updated CPU usage
    let final_cpu = process.cpu_usage();
    let elapsed_time = initial_time.elapsed().as_secs_f64();
    // Calculate CPU usage percentage
    let cpu_usage_percent = (final_cpu - initial_cpu) / (elapsed_time as f32);

    cpu_usage_percent
}
