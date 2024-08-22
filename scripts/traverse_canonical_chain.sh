#!/bin/bash

# Check if the directory and starting file are provided as arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <directory> <starting_file>"
    exit 1
fi

# Directory and starting file passed as arguments
directory="$1"
starting_file="$2"

# Initial file path
current_file="$directory/$starting_file"

while [ -f "$current_file" ]; do
    # Print the current file name
    echo "$(basename "$current_file")"

    # Extract the previous state hash
    previous_hash=$(jq -r '.protocol_state.previous_state_hash' "$current_file")

    # Extract the height from the current filename
    height=$(basename "$current_file" | cut -d'-' -f2)

    # Decrement the height
    new_height=$((height - 1))

    # Construct the path for the next file
    next_file="$directory/mainnet-$new_height-$previous_hash.json"

    # Check if the next file exists
    if [ ! -f "$next_file" ]; then
        echo "File not found: $next_file"
        break
    fi

    # Update current_file to continue traversal
    current_file="$next_file"
done
