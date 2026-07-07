# Pinned to a specific bookworm-based tag (not `latest-rust-1`, which floats
# onto whatever Debian release cargo-chef currently publishes). The runtime
# stages below are `debian:bookworm-slim` - builder and runtime must stay on
# the same Debian release or glibc symbol versions can mismatch (e.g. `web`
# crashed with `GLIBC_2.38' not found` when the builder drifted to trixie).
FROM lukemathwalker/cargo-chef:0.1.77-rust-1.94.0-bookworm AS chef
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
RUN curl -L https://github.com/sass/dart-sass/releases/download/1.97.3/dart-sass-1.97.3-linux-x64.tar.gz | tar -xz \
    && mv dart-sass /usr/local/lib/dart-sass \
    && ln -s /usr/local/lib/dart-sass/sass /usr/local/bin/sass \
    && chmod +x /usr/local/bin/sass

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz | tar -xvz \
    && mv cargo-binstall /usr/local/cargo/bin/cargo-binstall

RUN cargo binstall cargo-leptos --version 0.3.6 -y
RUN cargo binstall wasm-bindgen-cli --version 0.2.121 -y
RUN rustup target add wasm32-unknown-unknown

COPY --from=planner /app/recipe.json recipe.json
# Cook Web Dependencies (Host & WASM)
RUN cargo chef cook --release --recipe-path recipe.json -p web --features ssr --no-default-features
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p web --features hydrate --no-default-features

COPY rust .
ENV SQLX_OFFLINE=true
WORKDIR /app/web
RUN cargo leptos build --release

# --- Builder for migrate ---
FROM chef AS migrate-builder
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz | tar -xvz \
    && mv cargo-binstall /usr/local/cargo/bin/cargo-binstall
RUN cargo binstall sqlx-cli --version 0.8.6 --no-confirm

# --- Final Stage for migrate ---
FROM debian:bookworm-slim AS migrate
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=migrate-builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY rust/web/migrations /migrations
CMD ["sqlx", "migrate", "run", "--source", "/migrations"]

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
FROM debian:bookworm-slim AS bot
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bot .
EXPOSE 4000
CMD ["./bot"]

FROM debian:bookworm-slim AS battleship-2
COPY --from=builder /app/target/release/battleship_2_http .
CMD ["./battleship_2_http"]

FROM debian:bookworm-slim AS acquire-1
COPY --from=builder /app/target/release/acquire_1_http .
CMD ["./acquire_1_http"]

FROM debian:bookworm-slim AS category-5-2
COPY --from=builder /app/target/release/category_5_2_http .
CMD ["./category_5_2_http"]

FROM debian:bookworm-slim AS lost-cities-1
COPY --from=builder /app/target/release/lost_cities_1_http .
CMD ["./lost_cities_1_http"]

FROM debian:bookworm-slim AS lost-cities-2
COPY --from=builder /app/target/release/lost_cities_2_http .
CMD ["./lost_cities_2_http"]

FROM debian:bookworm-slim AS liars-dice-2
COPY --from=builder /app/target/release/liars_dice_2_http .
CMD ["./liars_dice_2_http"]

FROM debian:bookworm-slim AS no-thanks-2
COPY --from=builder /app/target/release/no_thanks_2_http .
CMD ["./no_thanks_2_http"]

FROM debian:bookworm-slim AS greed-2
COPY --from=builder /app/target/release/greed_2_http .
CMD ["./greed_2_http"]

FROM debian:bookworm-slim AS farkle-2
COPY --from=builder /app/target/release/farkle_2_http .
CMD ["./farkle_2_http"]

FROM debian:bookworm-slim AS zombie-dice-2
COPY --from=builder /app/target/release/zombie_dice_2_http .
CMD ["./zombie_dice_2_http"]

FROM debian:bookworm-slim AS for-sale-2
COPY --from=builder /app/target/release/for_sale_2_http .
CMD ["./for_sale_2_http"]

FROM debian:bookworm-slim AS sushizock-2
COPY --from=builder /app/target/release/sushizock_2_http .
CMD ["./sushizock_2_http"]

FROM debian:bookworm-slim AS sushi-go-2
COPY --from=builder /app/target/release/sushi_go_2_http .
CMD ["./sushi_go_2_http"]
