use std::env;
use std::path::PathBuf;

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
            //Path to the LibTorch binaries on macOS (custom location)
            
            let libtorch_path = PathBuf::from(env::var("HOME").unwrap())
                .join(".pyano")
                .join("binaries");

            //Ensure the dynamic linker knows where to find the LibTorch libraries
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}",
                libtorch_path.display()
            );

            //Set DYLD_LIBRARY_PATH without overwriting if it already exists
            let current_dyld_path = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
            let new_dyld_path = format!("{}:{}", libtorch_path.display(), current_dyld_path);
            println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}", new_dyld_path);

            //Avoid adding duplicate `-ltorch` link argument
            println!("cargo:rustc-link-arg=-ltorch");
            println!("cargo:rustc-env=TORCH_USE_MPS=1");


            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=CoreML");

            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=Foundation");

            // // Link CoreFoundation for system-specific symbols
            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=CoreFoundation");

            // // Link QuartzCore for rendering support
            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=QuartzCore");

            // // Link libsystem (handles some platform-specific functions)
            // println!("cargo:rustc-link-lib=system");

            // // Avoid duplicate RPATH entries
            // println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libtorch_path.display());
        }
        _ => {}
    }
}
