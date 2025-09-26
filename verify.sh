#!/bin/bash

# List of commands to run
commands=("cargo fmt" "cargo clippy -- -D warnings" "cargo check" "cargo build" "cargo build --target x86_64-pc-windows-gnu" "cargo test")

# ANSI colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

for cmd in "${commands[@]}"; do
    echo -n "[ ] $cmd"
    output_file=$(mktemp)
    if eval "$cmd" > "$output_file" 2>&1; then
        echo -e "\r[${GREEN}✓${NC}] $cmd"
        rm "$output_file"
    else
        echo -e "\r[${RED}✗${NC}] $cmd"
        cat "$output_file"
        rm "$output_file"
        exit 1
    fi
done