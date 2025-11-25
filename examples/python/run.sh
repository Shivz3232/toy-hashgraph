#!/bin/bash
set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Check if we're already in a virtualenv
if [ -z "$VIRTUAL_ENV" ] && [ -z "$CONDA_PREFIX" ]; then
    if [ -d "$SCRIPT_DIR/.venv" ]; then
        echo "Activating existing virtualenv..."
        source "$SCRIPT_DIR/.venv/bin/activate"
    elif [ -d "$PROJECT_ROOT/.venv" ]; then
        echo "Activating existing virtualenv..."
        source "$PROJECT_ROOT/.venv/bin/activate"
    else
        echo "Creating virtualenv..."
        python3 -m venv "$SCRIPT_DIR/.venv"
        source "$SCRIPT_DIR/.venv/bin/activate"
    fi
fi

echo "Building toy-hashgraph-py..."
cd "$PROJECT_ROOT/toy-hashgraph-py"
maturin develop

echo "Running Python example..."
cd "$SCRIPT_DIR"
python src/main.py