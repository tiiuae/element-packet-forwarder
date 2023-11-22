#!/bin/bash

MAX_WARNING_COUNT=65
#!/bin/bash

# Run Clippy and capture the output
CLIPPY_OUTPUT=$(cargo clippy 2>&1)

# Extract the number of warnings using grep
WARNING_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -o 'warning: ' | wc -l)

# Extract the number of warnings using grep
ERROR_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -o 'error: ' | wc -l)

# Print the result
echo "Number of warnings in Clippy: $WARNING_COUNT"

if [ "$ERROR_COUNT" -gt 0 ]; then
  echo "Clippy found errors. Build failed."
  exit 1
else
  echo "Clippy error count: $ERROR_COUNT"
fi

if [ "$WARNING_COUNT" -gt $MAX_WARNING_COUNT ]; then
  echo "Clippy found warnings more than $MAX_WARNING_COUNT. Build failed."
  exit 1
else
  echo "Warning count: $WARNING_COUNT"
fi

exit 0
