FROM ubuntu:latest

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
    gcc \
    build-essential \
    curl \
 && rm -rf /var/lib/apt/lists/*

# Set the working directory in the container
WORKDIR /app

# Copy the local application code into the container
COPY . .
## Install the most recent version of the nightly Rust toolchain using rustup.
## Use the minimal profile to keep the image size down as much as possible.
## https://rustup.rs/
# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"