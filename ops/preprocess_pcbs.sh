#!/bin/bash

# Check if the correct number of arguments is provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <input_directory> <output_directory>"
    exit 1
fi

# Directory containing the original JSON files
INPUT_DIR="$1"

# Directory to save the processed JSON files
OUTPUT_DIR="$2"

# Create the output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Process each JSON file in the input directory
for file in "$INPUT_DIR"/*.json; do
    # Extract the base filename
    filename=$(basename "$file")

    # Use jq to safely remove the "proofs" field, handling nulls and missing fields
    jq -c '
    if .staged_ledger_diff? != null and .staged_ledger_diff.diff? != null then
        .staged_ledger_diff.diff |= map(
            if .completed_works? != null then
                .completed_works |= map(del(.proofs))
            else
                .
            end
        )
    else
        .
    end' "$file" > "$OUTPUT_DIR/$filename"

    # Optionally, you can echo the filename to show progress
    echo "Processed $filename"
done

echo "All files processed. Modified files are in $OUTPUT_DIR."
