use std::env;
use dirs::home_dir;

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").expect("Unable to get TARGET_OS");

    match os.as_str() {
        "linux" | "windows" => {
            if let Some(lib_path) = env::var_os("DEP_TCH_LIBTORCH_LIB") {
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath={}",
                    lib_path.to_string_lossy()
                );
            }
            println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
            println!("cargo:rustc-link-arg=-Wl,--copy-dt-needed-entries");
            println!("cargo:rustc-link-arg=-ltorch");
        }
        "macos" => {
            // Path to the LibTorch binaries on macOS (custom location)
            if let Some(home_dir) = home_dir() {
                let libtorch_path = home_dir.join(".pyano").join("binaries");
                let libtorch_path_str = libtorch_path.to_str().expect("Invalid libtorch path");

                // Ensure the dynamic linker knows where to find the LibTorch libraries
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath,{}",
                    libtorch_path.display()
                );

                // Set DYLD_LIBRARY_PATH without overwriting if it already exists
                let current_dyld_path = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
                let new_dyld_path = format!("{}:{}", libtorch_path.display(), current_dyld_path);
                println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}", new_dyld_path);

                // Avoid adding duplicate `-ltorch` link argument
                println!("cargo:rustc-link-arg=-ltorch");
                println!("cargo:rustc-env=TORCH_USE_MPS=1");
            } else {
                panic!("Home directory not found");
            }
        }
        _ => {}
    }
}
