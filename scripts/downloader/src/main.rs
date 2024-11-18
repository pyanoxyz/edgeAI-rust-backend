use std::fs::{ create_dir_all, File, metadata, read_to_string };
use std::io::{ BufWriter, Read, Write };
use std::path::Path;
use std::sync::{ Arc, Mutex };

use indicatif::{ ProgressBar, ProgressStyle, ProgressDrawTarget };
use reqwest::blocking::Client;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Deserialize, Serialize)]
struct FileInfo {
    name: String,
    path: String,
    url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonResponse {
    files: Vec<FileInfo>,
}

#[derive(Debug)]
struct DownloadResult {
    path: String,
    success: bool,
    error: Option<String>,
}

#[derive(Debug)]
enum DownloadError {
    Io(String),
    Request(String),
    Template(String),
    Other(String),
}

impl std::error::Error for DownloadError {}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Io(e) => write!(f, "IO error: {}", e),
            DownloadError::Request(e) => write!(f, "Request error: {}", e),
            DownloadError::Template(e) => write!(f, "Template error: {}", e),
            DownloadError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

// Implement From for TemplateError
impl From<indicatif::style::TemplateError> for DownloadError {
    fn from(err: indicatif::style::TemplateError) -> Self {
        DownloadError::Template(err.to_string())
    }
}

type BoxResult<T> = Result<T, DownloadError>;

fn create_progress_style() -> BoxResult<ProgressStyle> {
    Ok(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) [{bytes_per_sec}]"
            )?
            .progress_chars("#>-")
    )
}

fn verify_or_prepare_file(file_path: &str) -> BoxResult<()> {
    if let Some(parent) = Path::new(file_path).parent() {
        create_dir_all(parent).map_err(|e| DownloadError::Io(e.to_string()))?;
    }
    Ok(())
}

fn download_single_file(
    client: &Client,
    url: &str,
    file_path: &str,
    progress: Option<Arc<Mutex<ProgressBar>>>
) -> BoxResult<DownloadResult> {
    verify_or_prepare_file(file_path)?;

    // Check if the file already exists
    if let Ok(meta) = metadata(file_path) {
        // Fetch the content length from the server using a HEAD request
        if let Ok(response) = client.head(url).send() {
            if
                let Some(content_length) = response
                    .headers()
                    .get("content-length")
                    .and_then(|l| l.to_str().ok())
                    .and_then(|l| l.parse::<u64>().ok())
            {
                // Compare file size with the content length
                if meta.len() == content_length {
                    // println!("File {} already exists and matches the expected size, skipping download.", file_path);
                    if let Some(ref progress) = progress {
                        if let Ok(bar) = progress.lock() {
                            bar.inc(content_length as u64);
                        }
                    }
                    return Ok(DownloadResult {
                        path: file_path.to_string(),
                        success: true,
                        error: None,
                    });
                }
            }
        }
    }

    let mut response = client
        .get(url)
        .send()
        .map_err(|e| DownloadError::Request(e.to_string()))?;

    if !response.status().is_success() {
        return Ok(DownloadResult {
            path: file_path.to_string(),
            success: false,
            error: Some(format!("Failed to access URL: {}", response.status())),
        });
    }

    let file = File::create(file_path).map_err(|e| DownloadError::Io(e.to_string()))?;
    let mut writer = BufWriter::new(file);

    let mut buffer = vec![0; 8192];
    while let Ok(n) = response.read(&mut buffer) {
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n]).map_err(|e| DownloadError::Io(e.to_string()))?;

        if let Some(ref progress) = progress {
            if let Ok(bar) = progress.lock() {
                bar.inc(n as u64);
            }
        }
    }

    Ok(DownloadResult {
        path: file_path.to_string(),
        success: true,
        error: None,
    })
}

fn download_from_json(json_source: &str, is_url: bool) -> BoxResult<Vec<DownloadResult>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| DownloadError::Request(format!("Failed to build client: {}", e)))?;

    let response: JsonResponse = if is_url {
        client
            .get(json_source)
            .send()
            .map_err(|e| DownloadError::Request(format!("Failed to fetch JSON: {}", e)))?
            .json()
            .map_err(|e| DownloadError::Request(format!("Failed to parse JSON: {}", e)))?
    } else {
        read_local_json(json_source)?
    };
    // println!("Found {} files to download", response.files.len());

    // Calculate total size of all files
    let total_size: u64 = futures::executor::block_on(async {
        let mut total = 0;
        for file_info in &response.files {
            let full_file_path = format!("{}{}", file_info.path, file_info.name);
            if let Ok(meta) = metadata(&full_file_path) {
                // If the file exists, check if the size matches the server's content-length
                if let Ok(resp) = client.head(&file_info.url).send() {
                    if
                        let Some(content_length) = resp
                            .headers()
                            .get("content-length")
                            .and_then(|l| l.to_str().ok())
                            .and_then(|l| l.parse::<u64>().ok())
                    {
                        // Skip files that match the content length
                        if meta.len() != content_length {
                            total += content_length;
                        }
                    }
                }
            } else {
                // If the file doesn't exist, add its size to total
                if let Ok(resp) = client.head(&file_info.url).send() {
                    if
                        let Some(content_length) = resp
                            .headers()
                            .get("content-length")
                            .and_then(|l| l.to_str().ok())
                            .and_then(|l| l.parse::<u64>().ok())
                    {
                        total += content_length;
                    }
                }
            }
        }
        total
    });
    // println!("Total Size: {}", total_size);
    // let progress_bar = ProgressBar::with_draw_target(Some(content_length), ProgressDrawTarget::stdout());
    let progress_target = ProgressDrawTarget::term_like(Box::new(console::Term::buffered_stdout()));

    let progress_bar = Arc::new(
        Mutex::new(ProgressBar::with_draw_target(Some(total_size), progress_target))
    );
    // Arc::new(Mutex::new(ProgressBar::with_draw_target(Some(total_size), ProgressDrawTarget::stdout())));

    if let Ok(bar) = progress_bar.lock() {
        bar.set_style(create_progress_style()?);
    }

    let mut handles = vec![];

    for file_info in response.files {
        let client = client.clone();
        let progress = Arc::clone(&progress_bar);

        handles.push(
            std::thread::spawn(move || {
                // Check if the file exists and matches the size before downloading
                let full_file_path = format!("{}{}", file_info.path, file_info.name);
                if let Ok(meta) = metadata(&full_file_path) {
                    if let Ok(resp) = client.head(&file_info.url).send() {
                        if
                            let Some(content_length) = resp
                                .headers()
                                .get("content-length")
                                .and_then(|l| l.to_str().ok())
                                .and_then(|l| l.parse::<u64>().ok())
                        {
                            // If the file exists and matches the size, skip the download
                            if meta.len() == content_length {
                                // if let Ok(bar) = progress.lock() {
                                //     bar.inc(content_length); // Move progress bar forward by file size
                                // }
                                // println!("File {} already exists, skipping.", file_info.name);
                                return Ok(DownloadResult {
                                    path: file_info.path.clone(),
                                    success: true,
                                    error: None,
                                });
                            }
                        }
                    }
                }

                download_single_file(
                    &client,
                    &file_info.url,
                    &format!("{}{}", file_info.path, file_info.name),
                    Some(progress)
                )
            })
        );
    }

    let mut results = vec![];
    for handle in handles {
        if
            let Ok(result) = handle
                .join()
                .map_err(|_| DownloadError::Other("Thread panicked".to_string()))?
        {
            results.push(result);
        }
    }

    if let Ok(bar) = progress_bar.lock() {
        bar.finish_with_message("All downloads complete");
    }

    Ok(results)
}

fn read_local_json(file_path: &str) -> BoxResult<JsonResponse> {
    let content = read_to_string(file_path)
        .map_err(|e| DownloadError::Io(format!("Failed to read JSON file: {}", e)))?;

    serde_json
        ::from_str(&content)
        .map_err(|e| DownloadError::Request(format!("Failed to parse JSON: {}", e)))
}

fn main() -> BoxResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        return Err(
            DownloadError::Other(
                "Usage: program <json_url_or_path> or program <url> <file_path>\n\
             For JSON mode, prefix URLs with 'http://' or 'https://', local files will be read directly".to_string()
            )
        );
    }

    if args.len() == 2 {
        // JSON URL mode
        let json_source = &args[1];
        let is_url = json_source.starts_with("http://") || json_source.starts_with("https://");
        let results = download_from_json(json_source, is_url)?;

        let mut has_error = false;

        for result in results {
            if result.error.is_some() {
                has_error = true;
                break;
            }
        }

        if has_error {
            println!("❌ Error occurred during download please retry.");
        } else {
            println!("\n");
            println!("✅ Download Completed");
        }
    } else {
        // Single file mode
        let url = &args[1];
        let file_path = &args[2];

        // Get content length from server
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| DownloadError::Request(format!("Failed to build client: {}", e)))?;

        let response = client
            .head(url)
            .send()
            .map_err(|e| DownloadError::Request(format!("Failed to fetch file info: {}", e)))?;
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|l| l.to_str().ok())
            .and_then(|l| l.parse::<u64>().ok())
            .ok_or(DownloadError::Request("Failed to get content length".to_string()))?;

        let progress_target = ProgressDrawTarget::term_like(
            Box::new(console::Term::buffered_stdout())
        );

        let progress_bar = ProgressBar::with_draw_target(Some(content_length), progress_target);
        let progress_style = ProgressStyle::default_bar().template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) [{bytes_per_sec}]"
        );

        if let Ok(progress_style) = progress_style {
            progress_bar.set_style(progress_style);
        }

        let result = download_single_file(
            &client,
            url,
            file_path,
            Some(Arc::new(Mutex::new(progress_bar)))
        )?;
        let mut has_error = false;

        match result.error {
            Some(_err) => {
                println!("{}", _err);
                has_error = true;
            }
            None => {}
        }
        if has_error {
            println!("❌ Error occurred during download please retry.");
        } else {
            println!("✅ Download Completed");
        }
    }

    Ok(())
}
