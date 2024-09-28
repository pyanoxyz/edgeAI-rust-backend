use actix_web::{get, post, web, App, HttpServer, HttpResponse, Responder};
use pair_programmer::pair_programmer_api::pair_programmer_generate_steps;
use serde::{Deserialize, Serialize};
use env_logger::Env;
use log::info;
use dotenv::dotenv;
use std::env;
mod chats;  // Import the chats module
mod authentication;
mod utils;
mod session_manager;
mod request_type;
mod platform_variables;
mod database;
mod embeddings;
mod rerank;
mod prompt_compression;
mod history;
mod parser;
mod rag;
mod pair_programmer;

#[get("/")]
async fn hello() -> impl Responder {
    info!("Request received");
    "Hello, world!"
}

#[post("/health")]
async fn echo(req_body: String) -> impl Responder {
    format!("You sent: {}", req_body)
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
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[get("/run-script")]
async fn run_script() -> impl Responder {
    // Spawn the async task to run the llama.cpp server in a separate thread
    let _llama_thread = tokio::spawn(async {
        run_llama_server().await;
    });

    // Log that the script has been triggered
    println!("Main server thread running...");

    HttpResponse::Ok().body("Llama.cpp server started successfully!")
}

async fn run_llama_server() {
    // Command to run the llama.cpp shell script
    let mut child = Command::new("sh")
        .arg("./src/public/run-model.sh")  // Path to your llama.cpp script
        .stdout(std::process::Stdio::piped())  // Capture stdout
        .stderr(std::process::Stdio::piped())  // Capture stderr
        .spawn()
        .expect("Failed to start the llama.cpp server");

    // Capture and process the output in real-time
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap() {
        // Print the output from the shell script to the main thread stdout
        println!("Llama server log: {}", line);
    }

    // Wait for the process to finish (don't await child directly, use .wait().await)
    child.wait().await.expect("Failed to wait on llama.cpp server process");
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    // Access the environment variables
    let llm_server_url = env::var("LOCAL_URL").expect("LOCAL_URL not found");
    let temperature = env::var("TEMPERATURE").expect("TEMPERATURE not found");
    let cloud_execution_mode = env::var("CLOUD_EXECUTION_MODE").expect("API_KEY not found");

    println!("LLM Server URL: {}", llm_server_url);
    println!("Temperature: {}", temperature);
    println!("Cloud Execution Mode: {}", cloud_execution_mode);
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    //TODO: This is meant just for testing the Parsers for indexing code, Delete it 
    //when the rag will be live
    // let p = parser::parse_code::IndexCode::new();
    // let chunks = p.create_code_chunks("/Users/saurav/Programs/pyano/openzeppelin-contracts");
    HttpServer::new(move || {
        App::new()
            .service(hello) // Register the GET route
            .service(echo)  // Register the POST route
            .service(json_handler)  // Register the POST route for JSON
            .service(run_script)  // Register the GET route for running script
            .configure(chats::chat_fill_routes)  // Add chat_fill routes
            .configure(chats::chat_plain_routes)  // Add chat routes
            .configure(chats::chat_explain_routes)  // Add chat explain routes
            .configure(chats::chat_explain_routes)  // Add chat explain routes
            .configure(chats::chat_refactor_routes)  // Add chat refactor routes
            .configure(chats::chat_testcases_routes)  // Add chat testcases routes
            .configure(chats::chat_findbugs_routes)  // Add chatfindbugs routes
            .configure(chats::chat_docstring_routes)  // Add docstring routes
            .configure(history::histoy_register_routes)  // Add chat explain routes
            .configure(rag::code_rag_api::register_routes)  // Add chat explain routes
            .configure(pair_programmer::pair_programmer_api::register_routes)  // Add chat explain routes

    })
    .bind("localhost:52556")?
    .run()
    .await
}
