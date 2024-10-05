use std::env;
use std::fs;
use std::path::Path;
use dirs::home_dir;

fn main() {
    // Get the home directory
    if let Some(home_dir) = home_dir() {

        let profile = env::var("PROFILE").unwrap_or_default();

        if profile == "release" || profile == "debug" {

            // println!("cargo:rustc-link-search=native=/usr/lib");
            println!("cargo:rustc-link-lib=c++");
            // println!("cargo:rustc-link-arg=-stdlib=libc++");

            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=CoreML");
            // println!("cargo:rustc-link-arg=-framework");
            // println!("cargo:rustc-link-arg=Foundation");


            let libtorch_path = home_dir.join(".pyano").join("binaries");
            // let libtorch_path_str ="/opt/homebrew/Cellar/pytorch/2.4.1/";

            let libtorch_path_str = libtorch_path.to_str().expect("Invalid libtorch path");


            // // Set the environment variable for download before any checks
            // env::set_var("TORCH_HOME", libtorch_path_str);
            // env::set_var("LIBTORCH", libtorch_path_str);
            env::set_var("MACOSX_DEPLOYMENT_TARGET", "11.0");

        // // Add these lines:
        // println!("cargo:rustc-link-lib=framework=CoreML");
        // println!("cargo:rustc-link-lib=framework=Foundation");
            // Ensure that LibTorch is downloaded by tch via build.rs
            println!("cargo:rerun-if-changed=build.rs");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-search=native=/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/lib");
            // Check if LibTorch is already downloaded
            if !Path::new(&libtorch_path).exists() {
                // Create directories if they don't exist
                fs::create_dir_all(&libtorch_path).expect("Failed to create directories for libtorch");
            }

            // Set RPATH for linking at runtime
            println!("cargo:rustc-link-search=native={}/lib", libtorch_path_str);
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", libtorch_path_str);

            // Optional: add the logic to download manually if required
        }
        } else {
        // Skip C++ compilation during `cargo check`
        println!("Skipping C++ compilation during cargo check");
    }
}
