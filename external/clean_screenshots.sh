#!/bin/bash

BASE_DIR="$1"

find "$BASE_DIR" -mindepth 1 -maxdepth 1 -type d | while read -r subdir; do
    echo "Processing: $subdir"

    find "$subdir" -type f -iname '*.png' | while read -r file; do
        if convert "$file" -format "%[mean]" info: | grep -q '^0$'; then
            echo "Removing black image: $file"
            rm -f "$file"
        fi
    done

    fdupes -rdN "$subdir"
done