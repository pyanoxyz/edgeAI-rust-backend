#!/bin/bash

# Set the DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH="$HOME/.pyano/bin/libtorch/lib:$DYLD_LIBRARY_PATH"

# Run the Rust binary (replace `target/debug/your_binary` with your actual binary path)
exec $HOME/.pyano/pyano-server

