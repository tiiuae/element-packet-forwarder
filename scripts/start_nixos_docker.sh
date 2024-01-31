#!/bin/bash

# Check if the path argument is provided
#if [ -z "$1" ]; then
#  echo "Usage: $0 <host_path>"
#  exit 1
#fi

# Assign the provided path to a variable
host_path="$PWD/.."

# Run the Docker container with the provided path
docker run --rm -it -v "$host_path":/prj element-packet-forwarder-docker:latest
