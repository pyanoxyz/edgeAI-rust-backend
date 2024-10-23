#!/bin/bash

# Set the DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH="$HOME/.pyano/bin/libtorch/lib:$DYLD_LIBRARY_PATH"

# Run the Rust binary (replace `target/debug/your_binary` with your actual binary path)

# Kill pyano-server if already running
function kill-pyano-server() {
    if pgrep -f pyano-server > /dev/null; then
        echo "Old server process found, killing it..."
        killall pyano-server
    fi
}
# Kill llama-server if already running
function kill-llama-server() {
    if pgrep -f llama-server > /dev/null; then
        echo "Old llama process found, killing it..."
        killall llama-server
    fi
}

kill-llama-server
kill-pyano-server


exec $HOME/.pyano/pyano-server

