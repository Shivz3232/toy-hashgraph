#!/bin/bash
set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building toy-hashgraph-js WASM package..."
cd "$PROJECT_ROOT/toy-hashgraph-js"
wasm-pack build --target bundler --out-name toy_hashgraph

echo "Installing dependencies..."
cd "$SCRIPT_DIR"
npm install

echo "Starting Vite dev server..."
npm run dev