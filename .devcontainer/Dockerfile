# Use official Rust image based on Debian
FROM rust:1.88-bookworm

# Install system dependencies required for building
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y \
    cmake \
    g++ \
    build-essential \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && apt-get update \
    && apt-get install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install rust-analyzer and other useful tools
RUN rustup component add clippy rustfmt rust-analyzer

# Set the default user
USER $USERNAME

# Set working directory
WORKDIR /workspaces/contact-detector
