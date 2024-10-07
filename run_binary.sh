#!/bin/bash

# Set the DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH="$HOME/.pyano/binaries/lib:$DYLD_LIBRARY_PATH"

# Run the Rust binary (replace `target/debug/your_binary` with your actual binary path)
exec target/aarch64-apple-darwin/release/pyano_server

