#!/bin/bash

# Check if the correct number of arguments is provided
if [ "$#" -ne 3 ]; then
    echo "Usage: $0 FONT_DIR FONT_FILE FONT_URL"
    exit 1
fi

# Assign command-line arguments to variables
FONT_DIR="$1"
FONT_FILE="$2"
FONT_URL="$3"

# Full path to the font file
FONT_PATH="$FONT_DIR/$FONT_FILE"

# Check if the font file already exists
if [ -f "$FONT_PATH" ]; then
    echo "Font file already exists at $FONT_PATH"
else
    echo "Font file not found. Downloading..."
    # Create the font directory if it doesn't exist
    mkdir -p "$FONT_DIR"
    # Download the font file
    curl -L -o "$FONT_PATH" "$FONT_URL"
    if [ $? -eq 0 ]; then
        echo "Font downloaded successfully to $FONT_PATH"
        exit 0
    else
        echo "Failed to download the font."
        exit 1
    fi
fi
