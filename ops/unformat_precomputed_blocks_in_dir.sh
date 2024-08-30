#!/bin/bash

# Check if the directory is provided as an argument
if [ -z "$1" ]; then
  echo "Usage: $0 <directory>"
  exit 1
fi

# Directory containing JSON files
DIRECTORY="$1"

# Find and iterate over each JSON file in the directory and subdirectories
find "$DIRECTORY" -type f -name "*.json" | while read -r file; do
  # Use jq to format the JSON as a single line
  jq -c . "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  echo "Processed: $file"
done
