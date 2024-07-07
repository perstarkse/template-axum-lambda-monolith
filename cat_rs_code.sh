#!/bin/bash

# Find all .rs files in the current directory and subdirectories
# Then cat the contents of each file
find . -type f -name "*.rs" -print0 | while IFS= read -r -d '' file; do
    echo "=== Contents of $file ==="
    cat "$file"
    echo "=== End of $file ==="
    echo
done

