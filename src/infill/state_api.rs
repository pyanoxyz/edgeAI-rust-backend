use actix_web::{ get, web, HttpResponse, Error };
use serde_json::json;
use log::{ debug, info };
use std::sync::Arc;
use crate::model_state::model_process::kill_model_process;
use crate::infill::model_process::run_infill_server;

use crate::infill::state::InfillModelState;

pub fn infill_model_state_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(mode_state).service(run_model).service(kill_model).service(restart_model);
}

#[get("/infill-model-state")]
pub async fn mode_state(data: web::Data<Arc<InfillModelState>>) -> Result<HttpResponse, Error> {
    let model_process_guard = data.infill_model_process.lock().await;
    let model_pid_guard = data.infill_model_pid.lock().unwrap();

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
            HttpResponse::Ok().json(
                json!({
                "message": "Model not started", "running": false
            })
            )
        )
    }
}

#[get("/run-infill-model")]
async fn run_model(data: web::Data<Arc<InfillModelState>>) -> Result<HttpResponse, Error> {
    // Acquires an asynchronous lock on the model process state to ensure only one process is running at a time.
    let mut model_process_guard = data.infill_model_process.lock().await;

    // Acquires a synchronous lock on the model PID state, needed to check if a model is already running and to track the PID.
    let model_pid_guard = data.infill_model_pid.lock().unwrap();

    // Check if the model process is already running by verifying if the process handle exists.
    let model_running = model_process_guard.is_some();
    if model_running {
        return Ok(
            HttpResponse::Ok().json(
                json!({"message": "Model is already running", "pid": model_pid_guard.unwrap_or(0)})
            )
        );
    }

    // Define a callback function that will be invoked when the model process starts, capturing the process PID.
    let callback = {
        let data_clone = data.clone();
        move |pid: Option<u32>| {
            // The callback captures the PID of the newly started model process.
            let mut model_pid_guard = data_clone.infill_model_pid.lock().unwrap(); // Lock the PID to update it.
            *model_pid_guard = pid; // Update the PID in the shared state.
        }
    };

    let reset_state_callback = {
        // Reset model state incase model didn't start
        let data_clone = data.clone();
        move || {
            let data_clone = data_clone.clone();
            tokio::spawn(async move {
                let mut model_process_guard = data_clone.infill_model_process.lock().await;
                let mut model_pid_guard = data_clone.infill_model_pid.lock().unwrap();

                *model_process_guard = None;
                *model_pid_guard = None;
            })
        }
    };
    // Start the model process in the background using tokio's spawn to run it asynchronously.
    let handle = tokio::spawn(async { run_infill_server(callback, reset_state_callback).await });

    // Store the handle to the running process in the shared state so it can be tracked or stopped later.
    *model_process_guard = Some(handle);

    // Log that the script has been triggered
    info!("Infill server thread running...");

    Ok(HttpResponse::Ok().json(json!({"message": "Infill Model started"})))
}

#[get("/kill-infill-model")]
async fn kill_model(data: web::Data<Arc<InfillModelState>>) -> Result<HttpResponse, Error> {
    let mut model_process_guard = data.infill_model_process.lock().await;
    let mut model_pid_guard = data.infill_model_pid.lock().unwrap();

    // Early return if model is not running
    let parent_pid = match *model_pid_guard {
        Some(pid) => pid,
        None => {
            return Ok(HttpResponse::Ok().json(json!({"message": "No running infill model found"})));
        }
    };
    // Kill the child process
    if let Err(e) = kill_model_process(parent_pid).await {
        return Ok(
            HttpResponse::InternalServerError().json(
                json!({"message": format!("Failed to kill infill model process: {}", e)})
            )
        );
    }

    *model_pid_guard = None; // Reset the PID after killing the child process
    *model_process_guard = None;
    return Ok(HttpResponse::Ok().json(json!({"message": "Infill Model stopped successfully"})));
}

#[get("/restart-infill-model")]
async fn restart_model(data: web::Data<Arc<InfillModelState>>) -> Result<HttpResponse, Error> {
    // Kill the model if it's running

    let mut model_process_guard = data.infill_model_process.lock().await;
    let mut model_pid_guard = data.infill_model_pid.lock().unwrap();

    let _parent_pid = match *model_pid_guard {
        Some(pid) => pid,
        None => {
            return Ok(
                HttpResponse::BadRequest().json(json!({"message": "No running infill model found"}))
            );
        }
    };

    if let Some(pid) = *model_pid_guard {
        if let Err(e) = kill_model_process(pid).await {
            return Ok(
                HttpResponse::InternalServerError().json(
                    json!({"message": format!("Failed to kill the in fill model process: {}", e)})
                )
            );
        }
    }
    // Reset the state and start a new model
    *model_pid_guard = None;
    *model_process_guard = None;

    // Call the run_model logic to start the new model
    // Define a callback that stores the process PID in the shared state
    let callback = {
        let data_clone = data.clone();
        move |pid: Option<u32>| {
            let mut model_pid_guard = data_clone.infill_model_pid.lock().unwrap();
            *model_pid_guard = pid; // Store the PID in shared state
        }
    };
    let reset_state_callback = {
        let data_clone = data.clone();
        move || {
            let data_clone = data_clone.clone();
            tokio::spawn(async move {
                let mut model_process_guard = data_clone.infill_model_process.lock().await;
                let mut model_pid_guard = data_clone.infill_model_pid.lock().unwrap();
                println!("RESETING STATE");
                *model_process_guard = None;
                *model_pid_guard = None;
            })
        }
    };
    // Start the llama process and get the PID using the callback
    let handle = tokio::spawn(async { run_infill_server(callback, reset_state_callback).await });

    *model_process_guard = Some(handle);

    // Log that the script has been triggered
    println!("Main server thread running...");

    Ok(HttpResponse::Ok().json(json!({"message": "Infill Model started"})))
}
