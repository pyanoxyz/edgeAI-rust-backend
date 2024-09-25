use log::{info, debug, warn, error};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use walkdir::WalkDir;

use crate::parser::parser::ParserLoader;

// Define the struct for IndexCode
pub struct IndexCode{
    parse_loader: ParserLoader
}


impl IndexCode {
    pub fn new() -> Self {
       let parse_loader = ParserLoader::new();
       Self { parse_loader: parse_loader }
    }

    pub fn create_code_chunks(&self, repo_path: &str) -> Vec<Chunk> {
        let mut all_chunks: Vec<Chunk> = Vec::new();
        
        for entry in WalkDir::new(repo_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(chunks) = self.process_file(path) {
                    info!("Code chunk for {:?} length of chunks {:?}", path, chunks.len());
                    all_chunks.extend(chunks);
                } else {
                    info!("Skipping binary or unreadable file: {:?}", path);
                }
            }
        }
    
        debug!("Extracted {} code chunks", all_chunks.len());
        all_chunks
    }

    /// Static method to check if a file is binary
    /// A helper function to determine if the file is binary by reading a portion of it.
    fn is_binary_file(content: &[u8]) -> bool {
        // Check for non-printable (binary) characters in the content
        for &byte in content.iter().take(1024) {
            // ASCII printable characters range from 0x20 (space) to 0x7E (~)
            // ASCII control characters (below 0x20) are typically non-printable, except for newlines and tabs
            if !(byte == b'\n' || byte == b'\t' || (0x20 <= byte && byte <= 0x7E)) {
                return true; // Likely a binary file
            }
        }
        false // Likely a text file
    }

    fn process_file(&self, file_path: &Path) -> Option<Vec<Chunk>> {
        // Open the file at the given file path.
        let file = File::open(file_path).expect("Failed to open file");

        // Create a buffered reader to efficiently read the file's content.
        let mut reader = BufReader::new(file);

        // Create a mutable byte vector to store the file's content.
        let mut content = Vec::new();

        // Read the entire content of the file into the `content` vector.
        reader.read_to_end(&mut content).expect("Failed to read file");

        // Check if the file is binary
        if Self::is_binary_file(&content) {
            debug!("The file is binary and will not be chunked.");
            return None;
        }

        // Call the `chunk_code` method to process the content into chunks,
        // passing the file content and the file path to determine the chunking strategy.
        Some(self.chunk_code(&content, file_path))
    }


    pub fn chunk_code(&self, file_content: &[u8], file_path: &Path) -> Vec<Chunk> {
        // Convert the file content from a slice of bytes to a mutable vector of bytes.
        let file_content = file_content.to_vec();
        
        // Check if the file content is empty. If it is, log a warning and return an empty vector.
        if file_content.is_empty() {
            warn!("File content is empty for {:?}", file_path);
            return Vec::new();
        }
    
        // Get the language name based on the file path's extension.
        let lang_name = self.get_lang_name(file_path);
    
        // Log the detected language name.
        info!("Detected Language Name: {}", lang_name);
    
        // If the language is unknown, log the information and return the entire file as a single chunk.
        if lang_name == "unknown" {
            info!("Unknown file type. Returning whole file as a single chunk.");
            return vec![Chunk {
                chunk_type: "unknown".to_string(),  // Set chunk type as 'unknown'
                content: String::from_utf8_lossy(&file_content).to_string(),  // Gracefully handle invalid UTF-8
                start_line: 0,  // Set start line as 0
                end_line: file_content.iter().filter(|&&c| c == b'\n').count(),  // Count the number of lines by counting newlines
                file_path: file_path.to_str().unwrap().to_string(),  // Convert file path to a string
            }];
        }
    
        // Initialize the parser for the specific file type using the file path.
        let mut parser = self.parse_loader.get_parser(file_path).unwrap();
    
        // Parse the file content into a syntax tree. If parsing fails, panic with an error message.
        let tree = parser.parse(file_content.as_slice(), None).expect("Failed to parse file");
        
        // Get the root node of the parsed syntax tree.
        let root_node = tree.root_node();
        
        // Create an empty vector to store the chunks of code.
        let mut chunks = Vec::new();
    
        // Traverse the syntax tree and populate the chunks vector based on the file content and language.
        self.traverse(&root_node, &file_content, &mut chunks, file_path, &lang_name);
        
        // Return the vector of code chunks.
        chunks
    }
    
    fn traverse(&self, node: &tree_sitter::Node, file_content: &[u8], chunks: &mut Vec<Chunk>, file_path: &Path, lang_name: &str) {
        let chunk_types = self.get_chunk_types(lang_name);

        if chunk_types.contains(&node.kind().to_string()) {
            let content = String::from_utf8(file_content[node.start_byte()..node.end_byte()].to_vec()).unwrap();
            chunks.push(Chunk {
                chunk_type: node.kind().to_string(),
                content,
                start_line: node.start_position().row,
                end_line: node.end_position().row,
                file_path: file_path.to_str().unwrap().to_string(),
            });
        }

        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            self.traverse(&child, file_content, chunks, file_path, lang_name);
        }
    }

    fn get_lang_name(&self, file_path: &Path) -> String {
        match file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("unknown") {
            "sh" => "bash".to_string(),
            "sol" => "solidity".to_string(),
            "c" => "c".to_string(),
            "cpp" => "cpp".to_string(),
            "cs" => "c_sharp".to_string(),
            "css" => "css".to_string(),
            "dockerfile" => "dockerfile".to_string(),
            "ex" => "elixir".to_string(),
            "elm" => "elm".to_string(),
            "go" => "go".to_string(),
            "hs" => "haskell".to_string(),
            "html" => "html".to_string(),
            "java" => "java".to_string(),
            "js" => "javascript".to_string(),
            "json" => "json".to_string(),
            "jl" => "julia".to_string(),
            "lua" => "lua".to_string(),
            "mk" => "make".to_string(),
            "php" => "php".to_string(),
            "py" => "python".to_string(),
            "rb" => "ruby".to_string(),
            "rs" => "rust".to_string(),
            "scala" => "scala".to_string(),
            "sql" => "sql".to_string(),
            "ts" => "typescript".to_string(),
            "tsx" => "tsx".to_string(),
            "yaml" => "yaml".to_string(),
            "erl" => "erlang".to_string(),
            "kt" => "kotlin".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn get_chunk_types(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec!["function_definition", "class_definition", "module"],
            "javascript" => vec![
                "function_declaration", "method_definition", "arrow_function",
                "class_declaration", "program"
            ],
            "typescript" => vec![
                "function_declaration", "method_definition", "arrow_function", 
                "class_declaration", "interface_declaration", "program"
            ],
            "solidity" => vec![
                "function_definition", "contract_declaration", "struct_declaration", 
                "enum_declaration", "source_file"
            ],
            "rust" => vec![
                "function_item", "struct_item", "enum_item", 
                "impl_item", "source_file"
            ],
            "go" => vec![
                "function_declaration", "method_declaration", "struct_type", 
                "interface_type", "source_file"
            ],
            "java" => vec![
                "method_declaration", "class_declaration", "interface_declaration", 
                "program"
            ],
            "c" => vec![
                "function_definition", "struct_specifier", "enum_specifier", 
                "translation_unit"
            ],
            "cpp" => vec![
                "function_definition", "class_specifier", "struct_specifier", 
                "enum_specifier", "translation_unit"
            ],
            "ruby" => vec![
                "method", "class", "module", "program"
            ],
            "php" => vec![
                "function_definition", "method_declaration", "class_declaration", 
                "interface_declaration", "program"
            ],
            "c_sharp" => vec![
                "method_declaration", "class_declaration", "interface_declaration", 
                "struct_declaration", "enum_declaration", "namespace_declaration", 
                "compilation_unit"
            ],
            "scala" => vec![
                "def", "class", "object", "trait", "compilation_unit"
            ],
            "swift" => vec![
                "function_declaration", "class_declaration", "struct_declaration", 
                "enum_declaration", "protocol_declaration", "source_file"
            ],
            "kotlin" => vec![
                "function_declaration", "class_declaration", "object_declaration", 
                "interface_declaration", "source_file"
            ],
            "bash" => vec![
                "function_definition", "command", "if_statement", "for_statement", 
                "while_statement", "case_statement", "program"
            ],
            "html" => vec![
                "element", "document"
            ],
            "css" => vec![
                "ruleset", "at_rule", "stylesheet"
            ],
            "sql" => vec![
                "select_statement", "insert_statement", "update_statement", 
                "delete_statement", "create_table_statement", "create_view_statement", 
                "create_function_statement", "create_procedure_statement"
            ],
            "yaml" => vec![
                "block_mapping", "block_sequence", "document"
            ],
            "json" => vec![
                "object", "array"
            ],
            "dockerfile" => vec![
                "from_instruction", "run_instruction", "cmd_instruction", 
                "expose_instruction", "env_instruction", "add_instruction", 
                "copy_instruction", "entrypoint_instruction", "volume_instruction", 
                "user_instruction", "workdir_instruction"
            ],
            "elixir" => vec![
                "function_definition", "module_definition", "source_file"
            ],
            "elm" => vec![
                "function_declaration", "type_declaration", "module_declaration", 
                "file"
            ],
            "haskell" => vec![
                "function_declaration", "data_declaration", "type_declaration", 
                "class_declaration", "instance_declaration", "module"
            ],
            "julia" => vec![
                "function_definition", "struct_definition", "module_definition", 
                "source_file"
            ],
            "lua" => vec![
                "function_definition", "local_function", "table_constructor", 
                "chunk"
            ],
            "make" => vec![
                "rule", "variable_assignment", "makefile"
            ],
            "markdown" => vec![
                "heading", "paragraph", "code_block", "list", "document"
            ],
            "erlang" => vec![
                "function", "attribute", "module"
            ],
            _ => vec!["source_file"],  // Default chunk type for unsupported languages
        }
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    }
    
}

// Define a struct for the code chunks
#[derive(Debug)]
pub struct Chunk {
    pub chunk_type: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub file_path: String,
}
