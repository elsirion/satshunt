#!/bin/bash
set -e

# Create required directories
mkdir -p uploads
mkdir -p lightning_data

# Build the application
echo "Building SatShunt..."
cargo build --release

# Run the application
echo "Starting SatShunt..."
echo "Open http://localhost:3000 in your browser"
cargo run --release
