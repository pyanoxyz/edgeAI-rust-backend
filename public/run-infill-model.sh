#!/bin/bash


#this is the version of the compiled llama.cpp core, This is generally requires to support the new launched models.
VERSION="b3899" # Change this if the version changes
INSTALL_DIR="$HOME/.pyano"
#This is where the compiled version of llama.cpp will be unzeipped that has llama-server binary to run llama.cpp server.
BUILD_DIR="$INSTALL_DIR/build/bin"
#This is the directory where all the models will be kept
MODEL_DIR="$HOME/.pyano/models"
#Path of the model where the model being used is placed.

# Use environment variables or set default values
MODEL_NAME="${MODEL_NAME:-Qwen2.5-Coder-7B-Instruct-IQ2_M.gguf}"
MODEL_URL="${MODEL_URL:-https://huggingface.co/bartowski/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-1.5B-Instruct-Q8_0.gguf}"
CTX="${CTX_SIZE:-4192}"
GPU_LAYERS_OFFLOADING="${GPU_LAYERS_OFFLOADING:--1}"
BATCH_SIZE="${BATCH_SIZE:-512}"
MLOCK="${MLOCK:-true}"
MMAP="${MMAP:-false}"
MODEL_PATH="$MODEL_DIR/$MODEL_NAME"


# Function to download and unzip if the version is not present
download_and_unzip() {
    # Path to the llama-server binary
    Llama_Server_Path="$INSTALL_DIR/build/bin/llama-server"
    echo "checking for $Llama_Server_Path"

    # Check if llama-server binary exists
    if [[ ! -f "$Llama_Server_Path" ]]; then
        echo "llama-server not found. Downloading and unzipping the new version..."

        # Check if curl or wget is installed and set DOWNLOAD_CMD accordingly
        if command -v curl &> /dev/null; then
            DOWNLOAD_CMD="curl -Lo"
        elif command -v wget &> /dev/null; then
            DOWNLOAD_CMD="wget -P"
        else
            echo "Neither curl nor wget is installed. Installing curl..."
            if [[ "$OSTYPE" == "linux-gnu"* ]]; then
                sudo apt-get update && sudo apt-get install -y curl
            elif [[ "$OSTYPE" == "darwin"* ]]; then
                brew install curl
            else
                echo "Unsupported OS for automatic curl installation. Please install curl or wget manually."
                exit 1
            fi
            DOWNLOAD_CMD="curl -Lo"
        fi

        # Create the ~/.pyano/ directory if it doesn't exist
        mkdir -p $INSTALL_DIR

        # Download the appropriate file based on the OS
        if [[ ! -f "$INSTALL_DIR/$ZIP_FILE" ]]; then
            echo "Downloading $ZIP_FILE for llama.cpp ..."
            $DOWNLOAD_CMD $INSTALL_DIR/$ZIP_FILE $DOWNLOAD_URL

            # Unzip the downloaded file
            echo "Unzipping $ZIP_FILE..."
            unzip $INSTALL_DIR/$ZIP_FILE -d $INSTALL_DIR/
        else
            echo "$ZIP_FILE already exists, skipping download and unzip."
        fi
    else
        echo "llama-server already exists at $Llama_Server_Path, skipping download."
    fi
}

set_download_info() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        ZIP_FILE="llama-$VERSION-bin-macos-arm64.zip"
        DOWNLOAD_URL="https://github.com/ggerganov/llama.cpp/releases/download/$VERSION/$ZIP_FILE"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        ZIP_FILE="llama-$VERSION-bin-ubuntu-x64.zip"
        DOWNLOAD_URL="https://github.com/ggerganov/llama.cpp/releases/download/$VERSION/$ZIP_FILE"
    else
        echo "Unsupported OS type: $OSTYPE"
        exit 1
    fi
}
# Function to install Python 3.12 using Homebrew (macOS) or package manager (Linux)
install_requirements_llama() {

    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update
        sudo apt-get install -y libgomp1
    fi

}


# Function to check if the model file is present and download it if not
check_and_download_model() {
    # Create the model directory if it doesn't exist
    mkdir -p "$MODEL_DIR"

    # Check if the model file exists
    if [[ ! -f "$MODEL_PATH" ]]; then
        echo "Model file $MODEL_NAME not found. Downloading..."

        # Download the model using your preferred method
        $HOME/.pyano/bin/downloader "$MODEL_URL" "$MODEL_PATH"

        echo "Model file downloaded to $MODEL_DIR/$MODEL_NAME."
    else
        echo "Model file $MODEL_NAME already exists in $MODEL_DIR."
    fi
}



# Ensure MODEL_PATH is set
if [ -z "$MODEL_PATH" ]; then
    echo "MODEL_PATH is not set. Please set the path to your model."
    exit 1
fi

# Download and unzip if necessary


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

check_and_download_model

# Set download info based on the OS
set_download_info

# Download and unzip the file

install_requirements_llama
download_and_unzip
get_num_cores
echo "Model being used $MODEL_PATH"
echo "Number of cores are  $num_cores"

# Run the server command
$BUILD_DIR/llama-server \
  -m $MODEL_PATH \
  --ctx-size $CTX \
  --parallel 2 \
  --n-gpu-layers $GPU_LAYERS_OFFLOADING\
  --port 52554
