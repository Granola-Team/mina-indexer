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
find "$INPUT_DIR" -type f -name '*.json' -size +30k -print0 | while IFS= read -r -d '' file; do
    # Extract the base filename
    filename=$(basename "$file")

    # Create a temporary file
    temp_file=$(mktemp)

    # Use jq to safely remove the "proofs" field, handling nulls and missing fields
    if jq -c '
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
    end' "$file" > "$temp_file"; then
        # If jq command was successful, move the temp file to the output
        mv "$temp_file" "$OUTPUT_DIR/$filename"
        echo "Processed $filename"
    else
        # If jq command failed, report the error and remove the temp file
        echo "Error processing $filename"
        rm "$temp_file"
    fi
done

echo "All files processed. Modified files are in $OUTPUT_DIR."
