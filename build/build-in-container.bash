#!/usr/bin/env bash

set -Eeuo pipefail

cd "$(git rev-parse --show-toplevel)"

CONTAINER_ID=$(docker run --user build -d builder /bin/sleep 86400)

BUILD_CONTEXT=(
    ".cargo/"
    "src/"
    "build.rs"
    "Cargo.lock"
    "Cargo.toml"
    "rust-toolchain.toml"
)

# Copy the build context over
docker exec "$CONTAINER_ID" mkdir -p /build/auth-rs
tar -c "${BUILD_CONTEXT[@]}" | docker cp - "$CONTAINER_ID":/build/auth-rs

# Build `auth-rs`
docker exec --workdir /build/auth-rs "$CONTAINER_ID" cargo build --release

# Copy it out to the host
mkdir -p dist
docker cp "$CONTAINER_ID":/build/auth-rs/target/release/auth-rs dist/auth-rs
