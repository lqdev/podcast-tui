FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Install additional packages for audio development
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    # Audio libraries for cross-platform development
    libasound2-dev \
    libpulse-dev \
    libjack-dev \
    # Additional development tools
    pkg-config \
    build-essential \
    curl \
    wget \
    git \
    # Terminal emulators for testing
    xterm \
    # Audio utilities for testing
    alsa-utils \
    pulseaudio-utils \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install additional Rust tools
RUN rustup component add clippy rustfmt \
    && cargo install cargo-watch cargo-expand cargo-audit

# Set up working directory
WORKDIR /workspace

# Create non-root user
RUN groupadd --gid 1000 vscode \
    && useradd --uid 1000 --gid vscode --shell /bin/bash --create-home vscode \
    && echo "vscode ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER vscode