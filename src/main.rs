use actix_cors::Cors;
use actix_web::{ get, post, web, App, HttpServer, Responder };
use serde::{ Deserialize, Serialize };
use env_logger::Env;
use log::info;
use dotenv::dotenv;
use std::sync::{ Arc, Mutex };
use tokio::sync::Mutex as TokioMutex; // Import tokio's async mutex
use reqwest::Client;

mod chats; // Import the chats module
mod authentication;
mod utils;
mod session_manager;
mod platform_variables;
mod database;
mod embeddings;
mod rerank;
mod prompt_compression;
mod parser;
mod rag;
mod pair_programmer;
mod summarization;
mod model_state;
mod llm_stream;
mod context;
mod infill;
use crate::model_state::state::ModelState;
use crate::infill::state::InfillModelState;

#[get("/")]
async fn hello() -> impl Responder {
    info!("Request received");
    "Hello, world!"
}

#[get("/health")]
async fn echo() -> impl Responder {
    "{\"status\": \"ok\"}"
}

#[derive(Deserialize)]
struct Info {
    name: String,
}

#[derive(Serialize)]
struct Greeting {
    message: String,
}

#[post("/json")]
async fn json_handler(info: web::Json<Info>) -> impl Responder {
    let response = Greeting {
        message: format!("Hello, {}!", info.name),
    };

    web::Json(response) // Respond with JSON
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let client = Client::new();
    let log_level = "info";
    info!("Setting LOG_LEVEL : {}", log_level);
    std::env::set_var("RUST_LOG", log_level); // Adjust the log level as needed

    let model_state = Arc::new(ModelState {
        model_pid: Arc::new(Mutex::new(None)),
        model_process: Arc::new(TokioMutex::new(None)),
    });

    let infill_model_state: Arc<InfillModelState> = Arc::new(InfillModelState {
        infill_model_pid: Arc::new(Mutex::new(None)),
        infill_model_process: Arc::new(TokioMutex::new(None)),
    });

    // Access the environment variables
    let llm_server_url = crate::utils::get_local_url();
    let temperature = crate::utils::get_llm_temperature();
    let cloud_execution_mode = crate::utils::is_cloud_execution_mode();

    info!("LLM Server URL: {}", llm_server_url);
    info!("Temperature: {}", temperature);
    info!("Cloud Execution Mode: {}", cloud_execution_mode);
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    //TODO: This is meant just for testing the Parsers for indexing code, Delete it
    //when the rag will be live
    // let p = parser::parse_code::IndexCode::new();
    // let chunks = p.create_code_chunks("/Users/saurav/Programs/pyano/openzeppelin-contracts");
    HttpServer::new(move || {
        let expose_headers = [
            "X-Session-ID",
            "X-Pair-Programmer-id",
            "access-control-allow-origin",
            "content-type",
        ];
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(expose_headers);

        App::new()
            .app_data(web::Data::new(model_state.clone()))
            .app_data(web::Data::new(infill_model_state.clone()))
            .app_data(web::Data::new(client.clone())) // Add client to state
            .wrap(cors)
            .service(hello) // Register the GET route
            .service(echo) // Register the POST route
            .service(json_handler) // Register the POST route for JSON
            .configure(model_state::model_state_api::model_state_routes)
            .configure(infill::model_state_api::infill_model_state_routes)
            .configure(chats::chat_infill_routes) // Add chat_fill routes
            .configure(chats::chat_plain_routes) // Add chat routes
            .configure(chats::chat_explain_routes) // Add chat explain routes
            .configure(chats::chat_refactor_routes) // Add chat refactor routes
            .configure(chats::chat_testcases_routes) // Add chat testcases routes
            .configure(chats::chat_findbugs_routes) // Add chatfindbugs routes
            .configure(chats::chat_docstring_routes) // Add docstring routes
            .configure(chats::chat_history_routes) // Add docstring routes
            .configure(rag::code_rag_api::register_routes) // Add chat explain routes
            .configure(pair_programmer::pair_programmer_api::register_routes) // Add chat explain routes
    })
        .bind("localhost:52556")?
        .run().await
}
