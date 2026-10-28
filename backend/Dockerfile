# Stage 1: Build
FROM rust AS builder

# Set the working directory
WORKDIR /usr/src/app

# Copy Cargo manifest files to leverage Docker caching
COPY Cargo.toml Cargo.lock ./
COPY bin/checkpoint-explorer/Cargo.toml bin/checkpoint-explorer/
# FIXME: we can optimize in terms of lines of code by using a wildcard
COPY database/Cargo.toml database/
COPY fullnode-client/Cargo.toml fullnode-client/
COPY migration/Cargo.toml migration/
COPY model/Cargo.toml model/

# Fetch dependencies first to leverage caching
RUN cargo fetch --locked

# Copy the project files into the container
COPY . .

# Build the project in release mode
RUN cargo build --locked

# Stage 2: Runtime
FROM rust

# Install required libraries for runtime (e.g., for PostgreSQL)
RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/app

# Copy the pre-built binaries from the build stage
COPY --from=builder /usr/src/app/target/debug/checkpoint-explorer .
COPY --from=builder /usr/src/app/target/debug/migration .
COPY .env* .
# Expose the application port (default is 3000 for checkpoint-explorer)
EXPOSE 3000

# Default command for the runtime container
CMD ["./checkpoint-explorer"]