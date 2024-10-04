use std::env;
use dirs::home_dir;

fn main() {
        // Path to the LibTorch binaries on macOS (custom location)
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.0");

        // Link necessary macOS system frameworks
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=CoreML");
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=Foundation");
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=CoreFoundation");

        if let Some(home_dir) = home_dir() {

            // Specify the SDK path
            let sdk_path = std::process::Command::new("xcrun")
                .args(&["--show-sdk-path"])
                .output()
                .expect("Failed to execute xcrun")
                .stdout;

            let sdk_path = String::from_utf8(sdk_path).unwrap().trim().to_string();
            println!("cargo:rustc-link-arg=-isysroot");
            println!("cargo:rustc-link-arg={}", sdk_path);


            let libtorch_path = home_dir.join(".pyano").join("binaries");
            let libtorch_path_str = libtorch_path.to_str().expect("Invalid libtorch path");

            // Ensure the dynamic linker knows where to find the LibTorch libraries
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libtorch_path_str);

            // Ensure the dynamic linker knows where to find the LibTorch libraries
            println!("cargo:rustc-link-search=native={}", libtorch_path_str);
            println!("cargo:rustc-link-search={}", libtorch_path_str); 


            // Set DYLD_LIBRARY_PATH without overwriting if it already exists
            let current_dyld_path = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
            let new_dyld_path = format!("{}:{}", libtorch_path.display(), current_dyld_path);
            println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}", new_dyld_path);

            // Avoid adding duplicate `-ltorch` link argument
            println!("cargo:rustc-link-arg=-ltorch");



            // Link static libraries
            println!("cargo:rustc-link-lib=static=XNNPACK");
            println!("cargo:rustc-link-lib=static=clog");
            println!("cargo:rustc-link-lib=static=cpuinfo");
            println!("cargo:rustc-link-lib=static=cpuinfo_internals");

            println!("cargo:rustc-link-lib=static=fmt");
            println!("cargo:rustc-link-lib=static=foxi_loader");
            println!("cargo:rustc-link-lib=static=gloo");
            println!("cargo:rustc-link-lib=static=kineto");
            println!("cargo:rustc-link-lib=static=nnpack");
            println!("cargo:rustc-link-lib=static=nnpack_reference_layers");
            
            println!("cargo:rustc-link-lib=static=onnx");
            println!("cargo:rustc-link-lib=static=onnx_proto");
            println!("cargo:rustc-link-lib=static=protobuf-lite");
            println!("cargo:rustc-link-lib=static=protobuf");
            println!("cargo:rustc-link-lib=static=protoc");

            println!("cargo:rustc-link-lib=static=pthreadpool");
            println!("cargo:rustc-link-lib=static=pytorch_qnnpack");
            println!("cargo:rustc-link-lib=static=tensorpipe");
            println!("cargo:rustc-link-lib=static=tensorpipe_uv");

            println!("cargo:rustc-link-lib=static=uv_a");
    

            // Link dynamic libraries
            println!("cargo:rustc-link-lib=dylib=torch_cpu"); // Add any other required libraries
            println!("cargo:rustc-link-lib=dylib=torch");
            println!("cargo:rustc-link-lib=dylib=c10"); // Add any other required libraries

            println!("cargo:rustc-link-lib=dylib=fbjni"); // Add any other required libraries

            println!("cargo:rustc-link-lib=dylib=omp"); // Add any other required libraries

            println!("cargo:rustc-link-lib=dylib=pytorch_jni"); // Add any other required libraries

            println!("cargo:rustc-link-lib=dylib=shm"); // Add any other required libraries


            println!("cargo:rustc-link-lib=dylib=torch_global_deps"); // Add any other required libraries

            println!("cargo:rustc-link-lib=dylib=torch_python"); // Add any other required libraries

                // println!("cargo:rustc-env=TORCH_USE_MPS=1");
            } else {
                panic!("Home directory not found");
            }
    
    }

