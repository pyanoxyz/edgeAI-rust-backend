use regex::Regex;
use std::error::Error;
use reqwest::Error as ReqwestError;
use std::fs;
use url::{Url, ParseError as UrlParseError};
use tempfile::TempDir;
use reqwest::Client;
use std::io::{self};
use std::path::Path;
use git2::Repository;
use log::info;
use crate::parser::parse_code::{ParseCode, Chunk};
use crate::database::db_config::DB_INSTANCE;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
#[derive(Debug)]
enum FileReadError {
    IoError(io::Error),
    ReqwestError(ReqwestError),
    UrlParseError(UrlParseError),
    FileNotFoundError(String),
}

#[derive(Debug)]
struct InvalidGitURLError(String);

impl std::fmt::Display for InvalidGitURLError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for InvalidGitURLError {}

fn clean_and_validate_git_url(url: &str) -> Result<String, Box<dyn Error>> {
    // Remove '/tree/master' or similar paths from the URL
    let re = Regex::new(r"/tree/[^/]+")?;
    let cleaned_url = re.replace_all(url, "").to_string();
    
    // Parse the URL
    let parsed_url = Url::parse(&cleaned_url)?;

    // Check if the URL ends with .git
    if !parsed_url.path().ends_with(".git") {
        return Err(Box::new(InvalidGitURLError(format!("The URL {} does not end with '.git'", cleaned_url))));
    }

    Ok(cleaned_url)
}


/// Downloads a GitHub repository as a ZIP file and extracts it to a temporary directory.
///
/// # Arguments
/// * `repo_url` - A GitHub repository URL (e.g., "https://github.com/owner/repo").
///
/// # Returns

/// The path to the temporary directory where the repo was cloned, or an error.
async fn download_github_repo(repo_url: &str, temp_dir: &TempDir) -> Result<String, Box<dyn std::error::Error>> {
    // Clean and validate the provided GitHub URL
    let validated_url = clean_and_validate_git_url(repo_url)?;
    let repo_dir = temp_dir.path().to_path_buf();

    // Create a temporary directory to clone the repository to

    // Use git2 to clone the repository into the temporary directory
    // let repo_dir = temp_dir.path().to_path_buf();
    
    // Clone the repository using git2
    Repository::clone(&validated_url, &repo_dir)?;

    // Return the path to the cloned repository
    Ok(repo_dir.to_string_lossy().to_string())
}



fn is_excluded_directory(dir_name: &str) -> bool {
    // List of common directories to exclude
    let excluded_dirs = vec![
        "node_modules",
        "bin",
        "include",
        "target",
        "build",
        "dist",
        "obj",
        "vendor",
        "venv",
        "env",
        "Pods",
        ".git",
        ".cache",
        "__pycache__",
    ];

    excluded_dirs.contains(&dir_name)
}

/// Recursively traverses the given directory path and appends all file paths to a list.
///
/// # Arguments
/// * `dir_path` - The path of the directory to traverse.
/// * `file_paths` - A mutable reference to a Vec<String> that will store the file paths.
///
/// # Returns
/// A `Result` indicating success or failure.
fn traverse_directory(dir_path: &str, file_paths: &mut Vec<String>) -> io::Result<()> {
    let path = Path::new(dir_path);

    // Check if the given path is a directory
    if path.is_dir() {
        // Read the directory entries (files and subdirectories)
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip excluded directories
            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    if is_excluded_directory(&dir_name.to_string_lossy()) {
                        continue; // Skip this directory
                    }
                }
                // Recurse into subdirectories
                traverse_directory(&path.to_string_lossy(), file_paths)?;
            }

            // If the path is a file, append it to the list
            if path.is_file() {
                file_paths.push(path.to_string_lossy().to_string());
            }
        }
    } else{
        info!("Couldnt find this directory {:?}", path);
    }

    Ok(())
}

/// Checks if the given path is a local directory.
fn is_local_directory(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir()
}

/// Checks if the given path is a remote repository by making an HTTP request.
async fn is_remote_repo(path: &str) -> Result<bool, Box<dyn Error>> {
    if let Ok(url) = Url::parse(path) {
        // Use `reqwest` to perform a HEAD request to the URL to see if it's reachable
        let client = Client::new();
        let response = client.head(url).send().await?;

        // Check for a successful status (like 200 OK or 301/302 redirects)
        if response.status().is_success() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn index_code(user_id: &str, session_id: &str, path: &str) -> Result<Vec<Chunk>, Box<dyn Error>> {
    let mut file_paths = Vec::new();
    let parse_code = ParseCode::new();
    let mut all_chunks: Vec<Chunk> = Vec::new();

    //Storing parent files in the database, before storing individual chunks for parent in 
    //another table
    // DB_INSTANCE.store_parent_context(user_id, session_id, path);
    // First, check if it's a local directory

    let mut filetype = "";
    let mut category = "";
    if is_local_directory(path) {
        filetype = "local";
        category = "files";
        traverse_directory(path, &mut file_paths)?;
        for file_path in &file_paths {
            let chunks = parse_code.process_local_file(file_path);
            all_chunks.extend(chunks.into_iter().flatten());

        }
    } 
    // Check if it's a local file
    else if Path::new(path).is_file() {
        filetype = "local_directory";
        category = "directories";
        // Add the file path directly to the list
        file_paths.push(path.to_string());
        println!("The path is a local file.");
        let chunks = parse_code.process_local_file(path);
        all_chunks.extend(chunks.into_iter().flatten());
    }
    
    // Check if it's a remote repository
    else if is_remote_repo(path).await? {
        filetype = "github_repo";
        category = "git_urls";
        let temp_dir = TempDir::new()?;

        let repo_dir_path = download_github_repo(path, &temp_dir).await?;
        info!("The repo is downloaded at {}", repo_dir_path);
        traverse_directory(&repo_dir_path, &mut file_paths)?;
        println!("The path is a remote repository.");
        for file_path in &file_paths {
            let chunks = parse_code.process_local_file(file_path);
            all_chunks.extend(chunks.into_iter().flatten());
        }
        
    } 
    // Check if it's a remote file
    else if path.starts_with("http://") || path.starts_with("https://") {
        filetype = "remote";
        category = "files";
        file_paths.push(path.to_string());
        let result = parse_code.process_remote_file(path).await?;
        // Check if the result is Some(Vec<Chunk>)
        if let Some(chunks) = result {
            // Extend `all_chunks` with the actual chunks
            all_chunks.extend(chunks.into_iter());
        }

    } 
    // If none of the conditions are met
    else {
        info!("The path is neither a local directory, file, remote repository, nor a remote file.");
    }
    DB_INSTANCE.store_parent_context(user_id, session_id, path, filetype, category);

    for chunk in &all_chunks {

        let tokens: Option<Vec<String>> = compress_chunk_content(chunk).await;
        let unwrapped_token = tokens.unwrap();
        let compressed_content = unwrapped_token.join(" ");        
        info!("content_tokens = {}, compressed_content_tokens={}", &chunk.content.len(), compressed_content.len());
        let embeddings: Option<Vec<f32>> = compressed_content_embeddings(&compressed_content).await;

        DB_INSTANCE.store_children_context(user_id, 
                        session_id, 
                        path, 
                        &chunk.chunk_type, 
                        &chunk.content, 
                        &compressed_content,
                        chunk.start_line, 
                        chunk.end_line, 
                        &chunk.file_path, 
                        &embeddings.unwrap() )
    }


    Ok(all_chunks)
}


async fn compressed_content_embeddings(content: &str) -> Option<Vec<f32>>{
    let embeddings_result = generate_text_embedding(content).await;
    let embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => return None,
    };
    Some(embeddings)
}


async fn compress_chunk_content (chunk: &Chunk) -> Option<Vec<String>>{
    let result: Result<Vec<String>, Box<dyn Error + Send + Sync>> = get_attention_scores(&chunk.content).await;
    let tokens = match result {
        Ok(tokens) => tokens,
        Err(e) =>  {println!("Error while unwrapping tokens: {:?}", e);
        return None
       }
    };
    Some(tokens)
}
