# Multi-stage Dockerfile for SwissPipe
# Stage 1: Frontend build
FROM node:22-alpine AS frontend-builder

WORKDIR /app/frontend

# Copy package files
COPY frontend/package*.json ./

# Install dependencies (including dev dependencies for build tools)
RUN npm ci

# Copy frontend source
COPY frontend/ .

# Build frontend
RUN npm run build-only

# Stage 2: Rust build
FROM rustlang/rust:nightly-bookworm AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built frontend assets to embed first
COPY --from=frontend-builder /app/frontend/dist ./dist/

# Copy all source files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the application in release mode
RUN cargo build --release --bin swisspipe

# Stage 3: Runtime image (distroless)
FROM gcr.io/distroless/cc-debian12

# Copy the binary from builder
COPY --from=rust-builder /app/target/release/swisspipe /usr/local/bin/swisspipe

# Set working directory
WORKDIR /app

# Create data directory with proper permissions
# Note: distroless doesn't have mkdir, so we'll create it at runtime
# The application will create the directory, but we need to ensure /app is writable
USER nonroot:nonroot

# Create a volume mount point for persistent data
VOLUME ["/app/data"]

# Expose port
EXPOSE 3700

# Run the binary
ENTRYPOINT ["/usr/local/bin/swisspipe"]