#!/bin/bash

VERSION="b3658" # Change this if the version changes
INSTALL_DIR="$HOME/.pyano"
BUILD_DIR="$INSTALL_DIR/build/bin"
MODEL_DIR="$HOME/.pyano/models"
MODEL_PATH="$MODEL_DIR/$MODEL_NAME"
MODEL_NAME=""
# MODEL_URL="https://huggingface.co/lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q8_0.gguf"

# Function to get system RAM in GB
get_system_ram() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        ram_gb=$(( $(sysctl -n hw.memsize) / 1024 / 1024 / 1024 ))
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        ram_gb=$(( $(grep MemTotal /proc/meminfo | awk '{print $2}') / 1024 / 1024 ))
    else
        echo "Unsupported OS type: $OSTYPE"
        exit 1
    fi
}

# Function to select model based on RAM
select_model() {
    get_system_ram
    if [ $ram_gb -lt 9 ]; then
        MODEL_NAME="qwen2.5-coder-1.5b-instruct-q8_0.gguf"
        MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-1.5b-instruct-q8_0.gguf"
        # MODEL_URL="https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q3_K_L.gguf"
        CTX=8192
        BATCH_SIZE=1024 #It's the number of tokens in the prompt that are fed into the model at a time. For example, if your prompt 
                        #is 8 tokens long at the batch size is 4, then it'll send two chunks of 4. It may be more efficient to process 
                        # in larger chunks. For some models or approaches, sometimes that is the case. It will depend on how llama.cpp handles it.
                        #larger BATCH size is speedup processing but more load on Memory
        GPU_LAYERS_OFFLOADED=16 #The number of layers to put on the GPU. The rest will be on the CPU. If you don't know how many layers there are, 
                                #you can use -1 to move all to GPU.

    elif [ $ram_gb -gt 24 ]; then
        MODEL_NAME="Meta-Llama-3.1-70B-Instruct-IQ1_M.gguf"
        MODEL_URL="https://huggingface.co/bartowski/Meta-Llama-3.1-70B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-70B-Instruct-IQ1_M.gguf"
        CTX=32768
        BATCH_SIZE=8192 #It's the number of tokens in the prompt that are fed into the model at a time. For example, if your prompt 
                        #is 8 tokens long at the batch size is 4, then it'll send two chunks of 4. It may be more efficient to process 
                        # in larger chunks. For some models or approaches, sometimes that is the case. It will depend on how llama.cpp handles it.
                        #larger BATCH size is speedup processing but more load on Memory        
        GPU_LAYERS_OFFLOADED=-1 #The number of layers to put on the GPU. The rest will be on the CPU. If you don't know how many layers there are, you can use -1 to move all to GPU.
    else

        # MODEL_NAME="Llama-3.1-SuperNova-Lite-Q6_K_L.gguf"
        # MODEL_URL="https://huggingface.co/bartowski/Llama-3.1-SuperNova-Lite-GGUF/resolve/main/Llama-3.1-SuperNova-Lite-Q6_K_L.gguf"

        MODEL_NAME="Qwen2.5-Coder-7B-Instruct-Q8_0.gguf"
        MODEL_URL="https://huggingface.co/bartowski/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-7B-Instruct-Q8_0.gguf"

        # MODEL_NAME="Qwen2.5-14B-Instruct-IQ3_XS.gguf"
        # MODEL_URL="https://huggingface.co/bartowski/Qwen2.5-14B-Instruct-GGUF/resolve/main/Qwen2.5-14B-Instruct-IQ3_XS.gguf"
        CTX=32768
        BATCH_SIZE=4096 #It's the number of tokens in the prompt that are fed into the model at a time. For example, if your prompt 
                        #is 8 tokens long at the batch size is 4, then it'll send two chunks of 4. It may be more efficient to process 
                        # in larger chunks. For some models or approaches, sometimes that is the case. It will depend on how llama.cpp handles it.
                        #larger BATCH size is speedup processing but more load on Memory        
        GPU_LAYERS_OFFLOADED=-1 #The number of layers to put on the GPU. The rest will be on the CPU. If you don't know how many layers there are, 
                        #you can use -1 to move all to GPU.
    fi
    MODEL_PATH="$MODEL_DIR/$MODEL_NAME"
}

select_model

# Function to determine OS and set the download URL and ZIP file name
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

# Install python3.12-venv and python3.12-dev on Ubuntu
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo apt-get install -y unzip
fi
# Function to check if the model file is present and download it if not
check_and_download_model() {
    # local model_file="Meta-Llama-3.1-8B-Instruct-Q8_0.gguf"

    # Create the model directory if it doesn't exist
    mkdir -p $MODEL_DIR

    # Check if the model file exists
    if [[ ! -f $MODEL_PATH ]]; then
        echo "Model file $MODEL_NAME not found. Downloading..."

        # Determine which download tool is available, and install wget if neither is found
        "$(dirname "$0")/download_file" $MODEL_URL $MODEL_PATH
        # if command -v curl &> /dev/null; then
        #     curl -Lo $MODEL_PATH $MODEL_URL
        # elif command -v wget &> /dev/null; then
        #     wget -O $MODEL_PATH $MODEL_URL
        # else
        #     echo "Neither curl nor wget is installed. Installing wget..."
        #     if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        #         sudo apt-get update && sudo apt-get install -y wget
        #     elif [[ "$OSTYPE" == "darwin"* ]]; then
        #         brew install wget
        #     else
        #         echo "Unsupported OS for automatic wget installation. Please install curl or wget manually."
        #         exit 1
        #     fi
        #     wget -O $MODEL_PATH $MODEL_URL
        # fi
        
        echo "Model file downloaded to $MODEL_DIR/$MODEL_NAME."
    else
        echo "Model file $MODEL_NAME already exists in $MODEL_DIR."
    fi
}


# Function to install Python 3.12 using Homebrew (macOS) or package manager (Linux)
install_requirements_llama() {

    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update
        sudo apt-get install -y libgomp1
    fi

}



# Function to download and unzip if the version is not present
download_and_unzip() {
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
        echo "Downloading $ZIP_FILE..."
        $DOWNLOAD_CMD $INSTALL_DIR/$ZIP_FILE $DOWNLOAD_URL

        # Unzip the downloaded file
        echo "Unzipping $ZIP_FILE..."
        unzip $INSTALL_DIR/$ZIP_FILE -d $INSTALL_DIR/
    else
        echo "$ZIP_FILE already exists, skipping download and unzip."
    fi
}

check_and_download_model

# Set download info based on the OS
set_download_info

# Download and unzip the file
download_and_unzip
install_requirements_llama

# Ensure MODEL_PATH is set
if [ -z "$MODEL_PATH" ]; then
    echo "MODEL_PATH is not set. Please set the path to your model."
    exit 1
fi

# Download and unzip if necessary
download_and_unzip

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
  --ctx-size $CTX \
  --parallel 2 \
  --n-gpu-layers $GPU_LAYERS_OFFLOADED\
  --port 52555 \
  --threads $num_cores \
  --color \
  --metrics \
    --batch-size $BATCH_SIZE \
  --numa isolate \
  --mlock \
    --no-mmap \
    --conversation \
    --flash-attn \
  --cache-type-k f16 \
  --cache-type-v f16 \
   --prompt-cache-all \
   --repeat-last-n 64 \
   --repeat-penalty 1.3 \
   --top-k 40 \
   --top-p 0.9