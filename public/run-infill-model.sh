#!/bin/bash

INSTALL_DIR="$HOME/.pyano"
#This is where the compiled version of llama.cpp will be unzeipped that has llama-server binary to run llama.cpp server.
BUILD_DIR="$INSTALL_DIR/build/bin"
#This is the directory where all the models will be kept
MODEL_DIR="$HOME/.pyano/models"
#Path of the model where the model being used is placed.

# TODO add command line args
# Use environment variables or set default values
MODEL_NAME="${MODEL_NAME:-infill-coder.gguf}"
CTX="${CTX_SIZE:-8192}"
GPU_LAYERS_OFFLOADING="${GPU_LAYERS_OFFLOADING:--1}"
BATCH_SIZE="${BATCH_SIZE:-512}"
MODEL_PATH="$MODEL_DIR/$MODEL_NAME"

# Ensure MODEL_PATH is set
if [ -z "$MODEL_PATH" ]; then
    echo "MODEL_PATH is not set. Please set the path to your model."
    exit 1
fi

# Calculate the number of CPU cores
get_num_cores() {
    if command -v nproc &> /dev/null; then
        # Linux
        num_cores=$(nproc)
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux fallback
        num_cores=$(grep -c ^processor /proc/cpuinfo)
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        num_cores=$(sysctl -n hw.ncpu)
    elif [[ "$OSTYPE" == "bsd"* ]]; then
        # BSD
        num_cores=$(sysctl -n hw.ncpu)
    else
        echo "Unsupported OS type: $OSTYPE"
        return 1
    fi
}

get_num_cores
echo "Model being used $MODEL_PATH"
echo "Number of cores are  $num_cores"

# Run the server command
$BUILD_DIR/llama-server \
  -m $MODEL_PATH \
  -np 8 \
  --ctx-size $CTX \
  --threads $num_cores \
  --parallel 4 \
  --n-gpu-layers $GPU_LAYERS_OFFLOADING\
  --port 52554
