FROM ubuntu:latest

ARG DEBIAN_FRONTEND=noninteractive
ARG USERNAME="vscode"
ARG USER_UID=1000
ARG USER_GID=$USER_UID


# Set the working directory
WORKDIR /prj



RUN apt-get update && apt-get install -y \
    gcc \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
 && rm -rf /var/lib/apt/lists/*

## Install the most recent version of the nightly Rust toolchain using rustup.
## Use the minimal profile to keep the image size down as much as possible.
## https://rustup.rvs/
# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup default nightly

RUN cargo install cargo-tarpaulin

