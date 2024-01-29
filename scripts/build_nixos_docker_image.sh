#!/bin/bash

#info: latest update on 2024-01-24, rust version 1.77.0-nightly (5d3d3479d 2024-01-23)
nix build '.#docker'
docker load < result
#image=$((docker load < result) | sed -n '$s/^Loaded image: //p')
#docker image tag "$image" element-packet-forwarder-docker:latest
#docker build -f ../Dockerfile.nixos -t element-packet-forwarder-docker .

exit 0