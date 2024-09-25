use libloading::{Library, Symbol};
use tree_sitter::{Parser, Language};
use log::info;
use std::path::Path;
use std::ffi::CString;

// Define function signatures for the `tree_sitter_LANGUAGE` functions.
type LanguageFn = unsafe fn() -> Language;

pub struct ParserLoader {
    pub lib: Library,
}


impl ParserLoader {
    pub fn new() -> Self {
        unsafe{
            //let current_dir: PathBuf = env::current_dir().expect("Failed to get current directory");

            // Construct the absolute path to the shared library
            let lib_path = "/Users/saurav/Programs/pyano/rust-backend/src/parser/languages.so";
            info!("Libpath is {:?}",lib_path);

            let lib = Library::new(lib_path).expect("Failed to load shared library");
            ParserLoader { lib }
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

            let cstr_parser_name = CString::new(symbol_name).map_err(|_| "Failed to create CString".to_string())?;
            let tree_sitter_lang: Symbol<LanguageFn> = self.lib.get(cstr_parser_name.as_bytes_with_nul())
                .map_err(|_| format!("Failed to load symbol for parser: {}", parser_name))?;
            
            // Set the language for the parser
            parser.set_language(tree_sitter_lang())
                .map_err(|_| "Failed to set language for the parser".to_string())?;
        }

        Ok(parser)
        }
}

// pub fn run_tree_sitter_parsers() {
//     // Load the shared library (language.so) dynamically

//     unsafe {
//         let current_dir: PathBuf = env::current_dir().expect("Failed to get current directory");

//         // Construct the absolute path to the shared library
//         let lib_path = "/Users/saurav/Programs/pyano/rust-backend/src/parser/languages.so";
//         info!("Libpath is {:?}",lib_path);

//         let lib = Library::new(lib_path).expect("Failed to load shared library");
//         // Get the symbols for the languages from the shared library
//         let tree_sitter_python: Symbol<LanguageFn> = lib.get(b"tree_sitter_python").expect("Failed to load tree_sitter_python symbol");
//         let tree_sitter_javascript: Symbol<LanguageFn> = lib.get(b"tree_sitter_javascript").expect("Failed to load tree_sitter_javascript symbol");
//         let tree_sitter_rust: Symbol<LanguageFn> = lib.get(b"tree_sitter_rust").expect("Failed to load tree_sitter_rust symbol");

//         // Create parsers for each language
//         let mut python_parser = Parser::new();
//         let mut js_parser = Parser::new();
//         let mut rust_parser = Parser::new();

//         // Set the language for each parser
//         python_parser.set_language(tree_sitter_python()).expect("Failed to set Python language");
//         js_parser.set_language(tree_sitter_javascript()).expect("Failed to set JavaScript language");
//         rust_parser.set_language(tree_sitter_rust()).expect("Failed to set Rust language");

//         // Now you can parse Python, JavaScript, and Rust using the same shared library.
//         // Example usage: Parse some code
//         let python_code = "def hello_world():\n    print('Hello, world!')";
//         let js_code = "function helloWorld() { console.log('Hello, world!'); }";
//         let rust_code = "fn hello_world() { println!(\"Hello, world!\"); }";

//         let python_tree = python_parser.parse(python_code, None).expect("Failed to parse Python code");
//         let js_tree = js_parser.parse(js_code, None).expect("Failed to parse JavaScript code");
//         let rust_tree = rust_parser.parse(rust_code, None).expect("Failed to parse Rust code");

//         // You can now work with the syntax trees for each language
//         println!("Python syntax tree: {:?}", python_tree);
//         println!("JavaScript syntax tree: {:?}", js_tree);
//         println!("Rust syntax tree: {:?}", rust_tree);
//     }
// }

// // use std::collections::HashMap;
// // use std::env;
// // use libloading::{Library, Symbol};
// // use std::path::PathBuf;
// // use tree_sitter::{Language, Parser};
// // use log::info;

// // // Define a type alias for the symbol function pointer
// // type LanguageFn = unsafe extern "C" fn() -> Language;

// pub fn run_tree_sitter_parsers() -> HashMap<String, Parser> {
//     // A mapping of language names to file extensions
//     let languages = vec![
//         ("tree_sitter_bash", ".sh"),
//         ("tree_sitter_c", ".c"),
//         ("tree_sitter_cpp", ".cpp"),
//         ("tree_sitter_c_sharp", ".cs"),
//         ("tree_sitter_css", ".css"),
//         ("tree_sitter_dockerfile", ".dockerfile"),
//         ("tree_sitter_elixir", ".ex"),
//         ("tree_sitter_elm", ".elm"),
//         ("tree_sitter_go", ".go"),
//         ("tree_sitter_haskell", ".hs"),
//         ("tree_sitter_html", ".html"),
//         ("tree_sitter_java", ".java"),
//         ("tree_sitter_javascript", ".js"),
//         ("tree_sitter_json", ".json"),
//         ("tree_sitter_julia", ".jl"),
//         ("tree_sitter_lua", ".lua"),
//         ("tree_sitter_make", ".mk"),
//         ("tree_sitter_markdown", ".md"),
//         ("tree_sitter_tree_sitter_php", ".php"),
//         ("tree_sitter_tree_sitter_python", ".py"),
//         ("tree_sitter_ruby", ".rb"),
//         ("tree_sitter_rust", ".rs"),
//         ("tree_sitter_scala", ".scala"),
//         ("tree_sitter_sql", ".sql"),
//         ("tree_sitter_typescript", ".ts"),
//         ("tree_sitter_tsx", ".tsx"),
//         ("tree_sitter_yaml", ".yaml"),
//         ("tree_sitter_erlang", ".erl"),
//         ("tree_sitter_kotlin", ".kt"),
//     ];

//     let mut parsers: HashMap<String, Parser> = HashMap::new();
//     let current_dir: PathBuf = env::current_dir().expect("Failed to get current directory");
//     info!("Current dir in parser {:?}", current_dir);
//     // Path to the shared library
//     let so_file_path = "/Users/saurav/Programs/pyano/rust-backend/src/parser/languages.so";
//     info!("Loading shared library from {:?}", so_file_path);

//     unsafe {
//         let lib = Library::new(so_file_path).expect("Failed to load shared library");

//         // Iterate over all languages and create parsers
//         for (lang, ext) in languages {
//             let symbol_name = format!("tree_sitter_{}", lang).into_bytes();
//             let language_fn: Symbol<LanguageFn> = lib.get(&symbol_name).expect(&format!("Failed to load symbol for {}", lang));

//             let mut parser = Parser::new();
//             parser.set_language(language_fn()).expect(&format!("Failed to set language for {}", lang));

//             parsers.insert(ext.to_string(), parser);
//         }
//     }

//     info!("Parsers for languages created successfully.");
//     parsers
// }
