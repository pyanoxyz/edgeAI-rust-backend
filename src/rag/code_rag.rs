use regex::Regex;
use std::error::Error;
use reqwest::Error as ReqwestError;
use std::fs;
use url::{Url, ParseError as UrlParseError};
use tokio::io::AsyncReadExt;
use std::io::Cursor;
use tempfile::TempDir;
use tokio::fs::File as TokioFile;
use reqwest::Client;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use git2::Repository;
use log::{info, debug};
use crate::parser::parse_code::{ParseCode, Chunk};
#[derive(Debug)]
enum FileReadError {
    IoError(io::Error),
    ReqwestError(ReqwestError),
    UrlParseError(UrlParseError),
    FileNotFoundError(String),
}

// // impl From<io::Error> for FileReadError {
// //     fn from(err: io::Error) -> FileReadError {
// //         FileReadError::IoError(err)
// //     }
// // }

// // impl From<ReqwestError> for FileReadError {
// //     fn from(err: ReqwestError) -> FileReadError {
// //         FileReadError::ReqwestError(err)
// //     }
// // }

// // impl From<UrlParseError> for FileReadError {
// //     fn from(err: UrlParseError) -> FileReadError {
// //         FileReadError::UrlParseError(err)
// //     }
// // }


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
    let repo = Repository::clone(&validated_url, &repo_dir)?;

    // Alternatively, you could use the Git command-line tool with tokio for async execution:
    // Command::new("git")
    //     .arg("clone")
    //     .arg(&validated_url)
    //     .arg(repo_dir.to_str().unwrap())
    //     .status()
    //     .await?;
    
    // Return the path to the cloned repository
    Ok(repo_dir.to_string_lossy().to_string())
}




/// Asynchronously downloads a file from either a local path or a remote URL and returns the content.
///
/// # Arguments
/// * `file_path` - A string slice that holds the file path or URL to read from.
///
/// # Returns
/// A `Result` containing a tuple of the file path (as `String`) and file contents (as `Vec<u8>`),
/// or a `FileReadError` if the operation fails.
async fn download_file(file_path: &str) -> Result<(String, Vec<u8>), FileReadError> {
    // Check if the file path is a URL (either HTTP or HTTPS)
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        // Parse the URL from the file path
        let mut url = Url::parse(file_path).map_err(FileReadError::UrlParseError)?;

        // If the URL is from GitHub and contains `/blob/`, replace `blob` with `raw` for direct file access
        if url.host_str() == Some("github.com") && url.path().contains("/blob/") {
            let path_segments: Vec<&str> = url.path().split('/').collect();
            let blob_index = path_segments.iter().position(|&x| x == "blob").unwrap_or(0);

            // Modify the path from blob to raw
            let mut modified_segments = path_segments.clone();
            modified_segments[blob_index] = "raw";
            let new_path = modified_segments.join("/");
            url.set_path(&new_path);
        }

        // Fetch the remote file using reqwest
        let client = Client::new();
        let response = client.get(url.as_str()).send().await.map_err(FileReadError::ReqwestError)?;

        // Check if the request was successful, if yes return content as Vec<u8>
        let response = response.error_for_status().map_err(FileReadError::ReqwestError)?;
        let content = response.bytes().await.map_err(FileReadError::ReqwestError)?.to_vec();
        return Ok((file_path.to_string(), content));
    } else {
        // If it's a local file path, check if the file exists and read its content asynchronously
        if std::path::Path::new(file_path).exists() {
            let mut file = TokioFile::open(file_path).await.map_err(FileReadError::IoError)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content).await.map_err(FileReadError::IoError)?;
            return Ok((file_path.to_string(), content));
        } else {
            // Return an error if the file is not found
            return Err(FileReadError::FileNotFoundError(file_path.to_string()));
        }
    }
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

pub async fn index_code(path: &str) -> Result<Vec<Chunk>, Box<dyn Error>> {
    let mut file_paths = Vec::new();
    let parse_code = ParseCode::new();
    let mut all_chunks: Vec<Chunk> = Vec::new();

    // First, check if it's a local directory
    if is_local_directory(path) {

        traverse_directory(path, &mut file_paths)?;
        for file_path in &file_paths {
            let chunks = parse_code.process_local_file(&file_path);
            all_chunks.extend(chunks.into_iter().flatten());
        }
    } 
    // Check if it's a local file
    else if Path::new(path).is_file() {
        // Add the file path directly to the list
        file_paths.push(path.to_string());
        println!("The path is a local file.");
        let chunks = parse_code.process_local_file(&path);
        all_chunks.extend(chunks.into_iter().flatten());
    }
    
    // Check if it's a remote repository
    else if is_remote_repo(path).await? {
        let temp_dir = TempDir::new()?;

        let repo_dir_path = download_github_repo(path, &temp_dir).await?;
        info!("The repo is downloaded at {}", repo_dir_path);
        traverse_directory(&repo_dir_path, &mut file_paths)?;
        println!("The path is a remote repository.");
        for file_path in &file_paths {
            let chunks = parse_code.process_local_file(&file_path);
            all_chunks.extend(chunks.into_iter().flatten());
        }
        
    } 
    // Check if it's a remote file
    else if path.starts_with("http://") || path.starts_with("https://") {
        file_paths.push(path.to_string());
        let result = parse_code.process_remote_file(&path).await?;
        // Check if the result is Some(Vec<Chunk>)
        if let Some(chunks) = result {
            // Extend `all_chunks` with the actual chunks
            all_chunks.extend(chunks.into_iter());
        }

    } 
    // If none of the conditions are met
    else {
        println!("The path is neither a local directory, file, remote repository, nor a remote file.");
    }

    // Handle file_paths (e.g., indexing, processing)
    // for file in &file_paths {
    //     println!("Indexed file: {}", file);
    // }

    Ok(all_chunks)
}

// - function that takes in remote repo and downlod the whole repo in a temp database

// - function that takes in local repo
// - walk the whole repo and collects all the files


// a function that takes in file path and returns file content

