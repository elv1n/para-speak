#!/bin/bash

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VENV_DIR="$SCRIPT_DIR/venv"

echo "==================================="
echo "Para-Speak Python Environment Setup"
echo "==================================="
echo ""

if [ -z "$PYTHON_PATH" ]; then
    echo "Auto-detecting Python..."
    
    for python_candidate in \
        "/opt/homebrew/bin/python3.13" \
        "/opt/homebrew/bin/python3.12" \
        "/opt/homebrew/bin/python3.11" \
        "/usr/local/bin/python3.13" \
        "/usr/local/bin/python3.12" \
        "/usr/local/bin/python3.11" \
        "$(which python3.13 2>/dev/null)" \
        "$(which python3.12 2>/dev/null)" \
        "$(which python3.11 2>/dev/null)" \
        "$(which python3 2>/dev/null)"
    do
        if [ -n "$python_candidate" ] && [ -x "$python_candidate" ]; then
            version=$("$python_candidate" -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>/dev/null)
            major=$(echo $version | cut -d. -f1)
            minor=$(echo $version | cut -d. -f2)
            
            if [ "$major" -eq 3 ] && [ "$minor" -ge 10 ]; then
                PYTHON_PATH="$python_candidate"
                echo "Found Python $version at: $PYTHON_PATH"
                break
            fi
        fi
    done
    
    if [ -z "$PYTHON_PATH" ]; then
        echo "Error: No suitable Python (3.10+) found!"
        echo "Please install Python 3.10 or later and try again."
        exit 1
    fi
else
    echo "Using Python from environment: $PYTHON_PATH"
fi

if [ -d "$VENV_DIR" ]; then
    echo ""
    echo "Virtual environment already exists at: $VENV_DIR"
    echo -n "Do you want to recreate it? (y/N): "
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "Removing existing virtual environment..."
        rm -rf "$VENV_DIR"
    else
        echo "Keeping existing virtual environment."
        echo -n "Do you want to update packages? (Y/n): "
        read -r update_response
        if [[ ! "$update_response" =~ ^[Nn]$ ]]; then
            echo ""
            echo "Updating packages..."
            source "$VENV_DIR/bin/activate"
            pip install --upgrade pip
            pip install -r "$SCRIPT_DIR/requirements.txt"
            echo ""
            echo "✅ Packages updated successfully!"
            exit 0
        else
            echo "Skipping package update."
            exit 0
        fi
    fi
fi

echo ""
echo "Creating virtual environment..."
"$PYTHON_PATH" -m venv "$VENV_DIR"

echo "Activating virtual environment..."
source "$VENV_DIR/bin/activate"

echo ""
echo "Upgrading pip..."
pip install --upgrade pip

echo ""
echo "Installing requirements..."
pip install -r "$SCRIPT_DIR/requirements.txt"

echo ""
echo "Verifying installation..."
python -c "import torch; print(f'✅ PyTorch {torch.__version__} installed')"
python -c "import mlx; print('✅ MLX installed')"
python -c "import parakeet_mlx; print('✅ Parakeet MLX installed')"
python -c "import librosa; print(f'✅ Librosa {librosa.__version__} installed')"
python -c "import soundfile; print('✅ Soundfile installed')"
python -c "import numpy; print(f'✅ NumPy {numpy.__version__} installed')"

echo ""
echo "==================================="
echo "✅ Setup completed successfully!"
echo "==================================="
echo ""
echo "Virtual environment created at: $VENV_DIR"
echo ""
echo "To use this environment:"
echo "  1. The run-para-speak.sh script will automatically use it"
echo "  2. Or manually activate: source $VENV_DIR/bin/activate"
echo ""