use libloading::{Library, Symbol};
use tree_sitter::{Parser, Language};
use log::info;
use std::path::Path;
use std::ffi::CString;

// Define function signatures for the `tree_sitter_LANGUAGE` functions.
type LanguageFn = unsafe fn() -> Language;

pub struct ParserLoader {
    pub lib: Library,
    pub sol_lib: Library
}


impl ParserLoader {
    pub fn new() -> Self {
        unsafe{
            //let current_dir: PathBuf = env::current_dir().expect("Failed to get current directory");

            // Construct the absolute path to the shared library
            let lib_path = "/Users/saurav/Programs/pyano/rust-backend/src/parser/languages.so";
            let sol_lib_path = "/Users/saurav/Programs/pyano/rust-backend/src/parser/solidity-language.so";

            info!("Libpath is {:?}",lib_path);

            let lib = Library::new(lib_path).expect("Failed to load shared library");
            let sol_lib =  Library::new(sol_lib_path).expect("Failed to load solidity library");
            ParserLoader { lib, sol_lib }
        }
    }

    pub fn get_parser(&self, file_path: &Path) -> Result<Parser, String>{
        let parser_name = match file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("unknown") {
            "sh" => "bash",
            "sol" => "solidity",
            "c" => "c",
            "cpp" => "cpp",
            "cs" => "c_sharp",
            "css" => "css",
            "dockerfile" => "dockerfile",
            "ex" => "elixir",
            "elm" => "elm",
            "go" => "go",
            "hs" => "haskell",
            "html" => "html",
            "java" => "java",
            "js" => "javascript",
            "json" => "json",
            "jl" => "julia",
            "lua" => "lua",
            "mk" => "make",
            "php" => "php",
            "py" => "python",
            "rb" => "ruby",
            "rs" => "rust",
            "scala" => "scala",
            "sql" => "sql",
            "ts" => "typescript",
            "tsx" => "tsx",
            "yaml" => "yaml",
            "erl" => "erlang",
            "kt" => "kotlin",
            _ => return Err(format!("Unsupported file extension for {:?}", file_path)),
        };
        let mut parser = Parser::new();

        // Dynamically load the language function from the library
        unsafe {
            // Use CString for null-terminated string compatibility
            let symbol_name = format!("{}_{}", "tree_sitter", parser_name);

            let cstr_parser_name = CString::new(symbol_name.clone()).map_err(|_| "Failed to create CString".to_string())?;

            let tree_sitter_lang: Symbol<LanguageFn>;
            if parser_name == "solidity"{
                tree_sitter_lang  = self.sol_lib.get(cstr_parser_name.as_bytes_with_nul())
                .map_err(|_| format!("Failed to load symbol solidity for parser: {}", parser_name))?;
            
            }else{
                tree_sitter_lang = self.lib.get(cstr_parser_name.as_bytes_with_nul())
                .map_err(|_| format!("Failed to load symbol for parser: {}", parser_name))?;
            
            }
            
            // Set the language for the parser
            parser.set_language(tree_sitter_lang())
                .map_err(|_| "Failed to set language for the parser".to_string())?;
        }

        Ok(parser)
        }
}
