use std::env;
use dirs::home_dir;

fn main() {
    // let home_dir = home_dir().expect("Unable to determine home directory");
    // let libtorch_path = home_dir.join(".pyano").join("binaries");
    // let libtorch_path_str = libtorch_path.to_str().expect("Invalid libtorch path");

    // // Tell cargo to pass the library search path
    // // Set the LIBTORCH environment variable to point to ~/.pyano/binaries
    // env::set_var("LIBTORCH",  libtorch_path_str);

    // println!("cargo:warning=Using libtorch path: {}", libtorch_path_str);

    // println!("cargo:rustc-link-search={}", libtorch_path_str); 
    // println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libtorch_path_str);

    // println!("cargo:rustc-link-search=native={}", libtorch_path_str);

    // Link static libraries
    // println!("cargo:rustc-link-lib=static=XNNPACK");
    // println!("cargo:rustc-link-lib=static=clog");
    // println!("cargo:rustc-link-lib=static=cpuinfo");
    // println!("cargo:rustc-link-lib=static=cpuinfo_internals");

    // println!("cargo:rustc-link-lib=static=fmt");
    // println!("cargo:rustc-link-lib=static=foxi_loader");
    // println!("cargo:rustc-link-lib=static=gloo");
    // println!("cargo:rustc-link-lib=static=kineto");
    // println!("cargo:rustc-link-lib=static=nnpack");
    // println!("cargo:rustc-link-lib=static=nnpack_reference_layers");
    
    // println!("cargo:rustc-link-lib=static=onnx");
    // println!("cargo:rustc-link-lib=static=onnx_proto");
    // println!("cargo:rustc-link-lib=static=protobuf-lite");
    // println!("cargo:rustc-link-lib=static=protobuf");
    // println!("cargo:rustc-link-lib=static=protoc");

    // println!("cargo:rustc-link-lib=static=pthreadpool");
    // println!("cargo:rustc-link-lib=static=pytorch_qnnpack");
    // println!("cargo:rustc-link-lib=static=tensorpipe");
    // println!("cargo:rustc-link-lib=static=tensorpipe_uv");

    // println!("cargo:rustc-link-lib=static=uv_a");
    

    // Link dynamic libraries
    // println!("cargo:rustc-link-lib=dylib=torch_cpu"); // Add any other required libraries
    // println!("cargo:rustc-link-lib=dylib=torch");
    // println!("cargo:rustc-link-lib=dylib=c10"); // Add any other required libraries

    // println!("cargo:rustc-link-lib=dylib=fbjni"); // Add any other required libraries

    // println!("cargo:rustc-link-lib=dylib=omp"); // Add any other required libraries

    // println!("cargo:rustc-link-lib=dylib=pytorch_jni"); // Add any other required libraries

    // println!("cargo:rustc-link-lib=dylib=shm"); // Add any other required libraries


    // println!("cargo:rustc-link-lib=dylib=torch_global_deps"); // Add any other required libraries

    // println!("cargo:rustc-link-lib=dylib=torch_python"); // Add any other required libraries

    // Link system frameworks
    // println!("cargo:rustc-link-lib=framework=CoreML");
    // println!("cargo:rustc-link-lib=framework=Foundation");
    // println!("cargo:rustc-link-lib=framework=CoreFoundation");

    // Update macOS version compatibility
    // println!("cargo:rustc-link-arg=-mmacosx-version-min=14.0");

    // Other linker flags
    // println!("cargo:rustc-link-arg=-Wl,-dead_strip_dylibs");

    // Enable MPS backend
    // println!("cargo:rustc-env=TORCH_USE_MPS=1");

}