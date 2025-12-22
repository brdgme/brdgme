FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
# Shared system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    nodejs \
    npm \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY rust .
RUN cargo chef prepare --recipe-path recipe.json

# --- Builder for Games (Standard Rust) ---
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook standard dependencies for the workspace
RUN cargo chef cook --release --recipe-path recipe.json
COPY rust .
# Build game services (exclude web to save time/resources here)
RUN cargo build --release --workspace --exclude web

# --- Builder for Web (Leptos) ---
FROM chef AS web-builder
# Install Web Tools (Sass, binstall, cargo-leptos, wasm target)
RUN curl -L https://github.com/sass/dart-sass/releases/download/1.77.8/dart-sass-1.77.8-linux-x64.tar.gz | tar -xz \
    && mv dart-sass /usr/local/lib/dart-sass \
    && ln -s /usr/local/lib/dart-sass/sass /usr/local/bin/sass \
    && chmod +x /usr/local/bin/sass

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz | tar -xvz \
    && mv cargo-binstall /usr/local/cargo/bin/cargo-binstall

RUN cargo binstall cargo-leptos --version 0.2.42 -y
RUN rustup target add wasm32-unknown-unknown

COPY --from=planner /app/recipe.json recipe.json
# Cook Web Dependencies (Host & WASM)
RUN cargo chef cook --release --recipe-path recipe.json -p web --features ssr --no-default-features
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p web --features hydrate --no-default-features

COPY rust .
ENV SQLX_OFFLINE=true
WORKDIR /app/web
RUN cargo leptos build --release

# --- Final Stage for Web ---
FROM debian:bookworm-slim AS web
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Copy the server binary from web-builder
COPY --from=web-builder /app/target/release/web ./brdgme_web
# Copy the site assets from web-builder
COPY --from=web-builder /app/target/site ./site
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 3000
CMD ["./brdgme_web"]

# --- Final Stages for Games ---
FROM debian:bookworm-slim AS acquire-1
COPY --from=builder /app/target/release/acquire_1_http .
CMD ["./acquire_1_http"]

FROM debian:bookworm-slim AS lost-cities-1
COPY --from=builder /app/target/release/lost_cities_1_http .
CMD ["./lost_cities_1_http"]

FROM debian:bookworm-slim AS lost-cities-2
COPY --from=builder /app/target/release/lost_cities_2_http .
CMD ["./lost_cities_2_http"]
