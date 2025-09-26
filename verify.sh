#!/bin/bash

# List of commands to run
commands=("cargo fmt" "cargo clippy" "cargo check" "cargo build" "cargo test")

# ANSI colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

for cmd in "${commands[@]}"; do
    output=$(eval "$cmd" 2>&1)
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $cmd"
    else
        echo -e "${RED}✗${NC} $cmd"
        echo "$cmd output:"
        echo "$output"
        exit 1
    fi
done