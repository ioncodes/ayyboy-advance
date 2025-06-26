#!/bin/bash

BASE_DIR="$1"

process_subdir() {
    subdir="$1"
    echo "Processing: $subdir"
    find "$subdir" -type f -iname '*.png' | while read -r file; do
        if convert "$file" -format "%[mean]" info: | grep -q '^0$'; then
            echo "Removing black image: $file"
            rm -f "$file"
        fi
    done
    fdupes -rdN "$subdir"
}

export -f process_subdir

find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | parallel -j40 process_subdir {}