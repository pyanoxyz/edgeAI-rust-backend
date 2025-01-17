use regex::Regex;
use std::error::Error;
use std::fs;
use url::Url;
use tempfile::TempDir;
use reqwest::Client;
use std::io::{ self };
use std::path::Path;
use git2::Repository;
use log::{ info, error, warn };
use crate::parser::parse_code::{ ParseCode, Chunk, ChunkWithCompressedData };
use crate::database::db_config::DB_INSTANCE;
use crate::embeddings::text_embeddings::generate_text_embedding;
use crate::prompt_compression::compress::get_attention_scores;
use crate::similarity_index::index::{ add_to_index, remove_from_index };
use rand::Rng;
use std::collections::HashSet;
use chrono::{ DateTime, Utc };
use std::time::SystemTime;
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
        return Err(
            Box::new(
                InvalidGitURLError(format!("The URL {} does not end with '.git'", cleaned_url))
            )
        );
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
async fn download_github_repo(
    repo_url: &str,
    temp_dir: &TempDir
) -> Result<String, Box<dyn std::error::Error>> {
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
        "__pycache__"
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
    } else {
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

pub async fn index_code(
    user_id: &str,
    session_id: &str,
    path: &str
) -> Result<Vec<Chunk>, Box<dyn Error>> {
    let mut file_paths = Vec::new();
    let parse_code = ParseCode::new();
    let mut all_chunks: Vec<Chunk> = Vec::new();
    let mut chunks_with_compressed_data: Vec<ChunkWithCompressedData> = Vec::new();

    //if this is empty which means the path is being indexed for the first time,
    // if not, then the path have been indexed earlier
    let if_already_index = DB_INSTANCE.fetch_path_session(user_id, session_id, path);
    //Storing parent files in the database, before storing individual chunks for parent in
    //another table
    // DB_INSTANCE.store_parent_context(user_id, session_id, path);
    // First, check if it's a local directory

    let mut filetype = "";
    let mut category = "";
    if is_local_directory(path) {
        warn!("Path has already been indexed {}", path);
        filetype = "local_directory";
        category = "directories";
        match if_already_index {
            Some(value) => {
                if let Some(timestamp) = value.get("timestamp") {
                    //all the files that has been changed since the last time the repo has been indexed
                    let modified_files = get_modified_files_since(
                        path,
                        timestamp.as_str().unwrap()
                    )?;
                    warn!(
                        "files that are modified since last indexed {:?} {}",
                        modified_files,
                        path
                    );

                    delete_index_only_files(user_id, session_id, modified_files.clone());
                    for file_path in &modified_files {
                        let chunks = parse_code.process_local_file(file_path);
                        all_chunks.extend(chunks.into_iter().flatten());
                    }
                    // You can now use `timestamp` for further processing here
                } else {
                    println!("Timestamp not found in the JSON value");
                }
            }
            None => {
                info!("Path is being indexed for the first time {}", path);

                //index all the files if this is the first time that the path is being indexed.
                traverse_directory(path, &mut file_paths)?;
                for file_path in &file_paths {
                    let chunks = parse_code.process_local_file(file_path);
                    all_chunks.extend(chunks.into_iter().flatten());
                }
            }
        }
    } else if
        // Check if it's a local file
        Path::new(path).is_file()
    {
        filetype = "local";
        category = "files";
        // Add the file path directly to the list
        file_paths.push(path.to_string());
        info!("The path = {} is a local file.", path);
        let chunks = parse_code.process_local_file(path);
        all_chunks.extend(chunks.into_iter().flatten());
    } else if
        // Check if it's a remote repository
        is_remote_repo(path).await?
    {
        filetype = "github_repo";
        category = "git_urls";
        let temp_dir = TempDir::new()?;

        let repo_dir_path = download_github_repo(path, &temp_dir).await?;
        info!("The repo is downloaded at {}", repo_dir_path);
        traverse_directory(&repo_dir_path, &mut file_paths)?;
        info!("The path is a remote repository.");
        for file_path in &file_paths {
            let chunks = parse_code.process_local_file(file_path);
            all_chunks.extend(chunks.into_iter().flatten());
        }
    } else if
        // Check if it's a remote file
        path.starts_with("http://") ||
        path.starts_with("https://")
    {
        filetype = "remote";
        category = "files";
        file_paths.push(path.to_string());
        let result = parse_code.process_remote_file(path).await?;
        // Check if the result is Some(Vec<Chunk>)
        if let Some(chunks) = result {
            // Extend `all_chunks` with the actual chunks
            all_chunks.extend(chunks.into_iter());
        }
    } else {
        // If none of the conditions are met
        info!("The path is neither a local directory, file, remote repository, nor a remote file.");
    }

    // Create a `HashSet` to track unique content
    let mut unique_chunks = HashSet::new();

    // Filter out duplicate chunks based on `content`, keeping the original `Chunk`
    all_chunks.retain(|chunk| unique_chunks.insert(chunk.content.clone()));

    DB_INSTANCE.store_parent_context(user_id, session_id, path, filetype, category);

    for chunk in &all_chunks {
        let tokens: Option<Vec<String>> = compress_chunk_content(chunk).await;
        let unwrapped_token = tokens.unwrap();
        let compressed_content = unwrapped_token.join(" ");
        info!(
            "content_tokens = {}, compressed_content_tokens={}",
            &chunk.content.len(),
            compressed_content.len()
        );
        // Unwrap the embeddings safely
        let chunk_id = generate_rowid();

        // Try to get the embeddings and update embeddings_vec
        if let Some(embeddings) = compressed_content_embeddings(&compressed_content).await {
            let chunk_with_data = ChunkWithCompressedData {
                chunk: chunk.clone(), // Assuming Chunk implements Clone
                compressed_content: compressed_content.clone(),
                embeddings: embeddings.clone(),
                chunk_id,
            };

            chunks_with_compressed_data.push(chunk_with_data);
        } else {
            error!("Failed to get embeddings for chunk: {:?}", chunk);
        }

        DB_INSTANCE.store_children_context(
            user_id,
            session_id,
            path,
            &chunk.chunk_type,
            &chunk.content,
            &compressed_content,
            chunk.start_line,
            chunk.end_line,
            &chunk.file_path,
            chunk_id
        );
    }

    add_to_index(session_id, chunks_with_compressed_data);
    info!("Updating the session context with path = {} with the latest timestamp", path);
    let _ = DB_INSTANCE.update_session_context_timestamp(user_id, session_id, path);
    Ok(all_chunks)
}

pub fn generate_rowid() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1_000_000_000_000_000..=9_999_999_999_999_999)
}

async fn compressed_content_embeddings(content: &str) -> Option<Vec<f32>> {
    let embeddings_result = generate_text_embedding(content).await;
    let embeddings = match embeddings_result {
        Ok(embeddings) => embeddings,
        Err(_) => {
            return None;
        }
    };
    Some(embeddings)
}

async fn compress_chunk_content(chunk: &Chunk) -> Option<Vec<String>> {
    let result: Result<Vec<String>, Box<dyn Error + Send + Sync>> = get_attention_scores(
        &chunk.content
    ).await;
    let tokens = match result {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("Error while unwrapping tokens: {:?}", e);
            return None;
        }
    };
    Some(tokens)
}

fn get_modified_files_since(dir: &str, timestamp_str: &str) -> io::Result<Vec<String>> {
    let mut modified_files = Vec::new();

    // Parse the RFC3339 timestamp string to a DateTime<Utc> and then to SystemTime
    let timestamp: SystemTime = DateTime::parse_from_rfc3339(timestamp_str)
        .expect("Failed to parse timestamp")
        .with_timezone(&Utc)
        .into();

    // Convert the string to a Path
    let path = Path::new(dir);

    // Traverse the directory
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        // Check if it's a file (not a directory)
        if file_path.is_file() {
            // Get the file metadata
            let metadata = fs::metadata(&file_path)?;

            // Get the last modified time
            if let Ok(modified) = metadata.modified() {
                // Compare with the parsed timestamp
                if modified > timestamp {
                    modified_files.push(file_path.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(modified_files)
}

pub fn delete_index(user_id: &str, session_id: &str, files: Vec<String>) {
    for file_path in files {
        match DB_INSTANCE.delete_parent_context(&file_path) {
            Ok(_) => {
                info!("Successfully deleted parent context for file: {:?}", file_path);
            }
            Err(e) => {
                error!("Error deleting {}", e.to_string());
            }
        }

        let vec_row_ids = match
            DB_INSTANCE.delete_children_context_by_parent_path(user_id, session_id, &file_path)
        {
            Ok(ids) => {
                info!("Successfully deleted chunks for file: {}", file_path);
                ids
            }
            Err(e) => {
                error!("Error deleting {}", e.to_string());
                Vec::new()
            }
        };
        info!("vec_row_ids that awere deleted from sqlite {:?}", vec_row_ids);
        remove_from_index(&session_id, vec_row_ids);
    }
}

pub fn delete_index_only_files(user_id: &str, session_id: &str, files: Vec<String>) {
    for file_path in files {
        let vec_row_ids = match
            DB_INSTANCE.delete_children_context_by_file_path(user_id, session_id, &file_path)
        {
            Ok(ids) => {
                info!("Successfully deleted chunks for file: {}", file_path);
                ids
            }
            Err(e) => {
                error!("Error deleting {}", e.to_string());
                Vec::new()
            }
        };
        info!("vec_row_ids that awere deleted from sqlite {:?}", vec_row_ids);
        remove_from_index(&session_id, vec_row_ids);
    }
}
