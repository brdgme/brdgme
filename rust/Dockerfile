# Short commit SHA, threaded into the web image as SENTRY_RELEASE
# (sha-<SHORT_SHA>, matching the image tag scheme in docker-bake.hcl/ci.yml).
# Must be re-declared with a bare `ARG SHORT_SHA` after each FROM that needs
# it - Docker only carries ARG values forward within the stage they're
# declared in.
ARG SHORT_SHA=dev

# Pinned to a specific bookworm-based tag (not `latest-rust-1`, which floats
# onto whatever Debian release cargo-chef currently publishes). The runtime
# stages below are `debian:bookworm-slim` - builder and runtime must stay on
# the same Debian release or glibc symbol versions can mismatch (e.g. `web`
# crashed with `GLIBC_2.38' not found` when the builder drifted to trixie).
FROM lukemathwalker/cargo-chef:0.1.77-rust-1.97.0-bookworm AS chef
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
ARG SHORT_SHA
# Install Web Tools (Sass, binstall, cargo-leptos, wasm target)
RUN curl -L https://github.com/sass/dart-sass/releases/download/1.101.0/dart-sass-1.101.0-linux-x64.tar.gz | tar -xz \
    && mv dart-sass /usr/local/lib/dart-sass \
    && ln -s /usr/local/lib/dart-sass/sass /usr/local/bin/sass \
    && chmod +x /usr/local/bin/sass

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz | tar -xvz \
    && mv cargo-binstall /usr/local/cargo/bin/cargo-binstall

RUN cargo binstall cargo-leptos --version 0.3.7 -y
RUN cargo binstall wasm-bindgen-cli --version 0.2.121 -y
RUN rustup target add wasm32-unknown-unknown

# wasm-split (getsentry/symbolicator release, pinned 26.6.0) - splits DWARF
# debug info out of the shipped wasm into a separate companion file for
# Sentry's Symbolicator to consume. Not a cargo tool; a single prebuilt
# glibc-dynamic ELF binary (libgcc_s already present in this base image).
RUN curl -L -o /usr/local/bin/wasm-split https://github.com/getsentry/symbolicator/releases/download/26.6.0/wasm-split-Linux-x86_64 \
    && chmod +x /usr/local/bin/wasm-split

# binaryen wasm-opt (pinned 129, the locally validated version) - used below
# to size-optimize the DWARF-stripped wasm. cargo-leptos downloads its own
# binaryen for its internal (DWARF-free) pipeline; we need one on PATH for
# the manual step. Only wasm-opt is copied out: binaryen ships an unrelated
# `wasm-split` binary that must not clobber the getsentry one above.
RUN curl -L https://github.com/WebAssembly/binaryen/releases/download/version_129/binaryen-version_129-x86_64-linux.tar.gz | tar -xz \
    && mv binaryen-version_129/bin/wasm-opt /usr/local/bin/wasm-opt \
    && rm -rf binaryen-version_129

COPY --from=planner /app/recipe.json recipe.json
# Cook Web Dependencies (Host & WASM)
RUN cargo chef cook --release --recipe-path recipe.json -p web --features ssr --no-default-features
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p web --features hydrate --no-default-features

COPY rust .
ENV SQLX_OFFLINE=true
WORKDIR /app/web
# Bundle the Sentry browser SDK to public/sentry.js before the leptos build,
# so its assets-dir sync (Cargo.toml [package.metadata.leptos]) ships it to
# site/sentry.js in the same pass.
RUN cd js && npm ci && npm run build
# wasm-opt (binaryen) SIGABRTs on any wasm carrying DWARF from wasm-bindgen
# --keep-debug (reproduced on binaryen 123 and 129, even at -O0), so
# cargo-leptos's monolithic `--wasm-debug` build cannot produce the Sentry
# debug companion. Instead: build WITHOUT --wasm-debug (the internal
# wasm-bindgen/wasm-opt pipeline never sees DWARF), which still produces the
# server binary, styles, assets and JS glue - then rebuild the shipped wasm
# manually below, splitting the DWARF out BEFORE wasm-opt runs.
RUN cargo leptos build --release
# Re-run wasm-bindgen on the raw wasm with --keep-debug so the DWARF (from
# `debug = true` on [profile.wasm-release]) survives into the output. Same
# wasm-bindgen version and input wasm as cargo-leptos's own run, so the JS
# glue already shipped to site/pkg matches; only the wasm is taken from here.
RUN wasm-bindgen --target web --keep-debug --no-typescript \
    --out-dir /tmp/wasm-dwarf --out-name web \
    /app/target/front/wasm32-unknown-unknown/wasm-release/web.wasm
# Split the DWARF into a companion file for Sentry before wasm-opt ever sees
# it: strips only .debug_* sections (the name section survives) and injects a
# build_id custom section for Sentry's debug-file matching (survives wasm-opt).
RUN wasm-split /tmp/wasm-dwarf/web_bg.wasm -o /tmp/wasm-dwarf/web_stripped.wasm \
    --strip --debug-out=/tmp/wasm-dwarf/web.debug
# Size-optimize the now-DWARF-free wasm over the top of the shipped web.wasm.
# Flags mirror wasm-opt-features in web/Cargo.toml; -g keeps the name section.
RUN wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int -g \
    /tmp/wasm-dwarf/web_stripped.wasm -o /app/target/site/pkg/web.wasm

# --- Debug companion for Web (DWARF for Sentry Symbolicator) ---
FROM scratch AS web-debug
COPY --from=web-builder /tmp/wasm-dwarf/web.debug /web.debug

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
ARG SHORT_SHA
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Copy the server binary from web-builder
COPY --from=web-builder /app/target/release/web ./brdgme_web
# Copy the site assets from web-builder
COPY --from=web-builder /app/target/site ./site
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="site"
ENV SENTRY_RELEASE="sha-${SHORT_SHA}"
EXPOSE 3000
CMD ["./brdgme_web"]

# --- Final Stages for Games ---
FROM debian:bookworm-slim AS bot
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bot .
EXPOSE 4000
CMD ["./bot"]

FROM debian:bookworm-slim AS operator
RUN apt-get -y update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/operator .
CMD ["./operator"]

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

FROM debian:bookworm-slim AS tic-tac-toe-2
COPY --from=builder /app/target/release/tic_tac_toe_2_http .
CMD ["./tic_tac_toe_2_http"]

FROM debian:bookworm-slim AS jaipur-2
COPY --from=builder /app/target/release/jaipur_2_http .
CMD ["./jaipur_2_http"]

FROM debian:bookworm-slim AS love-letter-2
COPY --from=builder /app/target/release/love_letter_2_http .
CMD ["./love_letter_2_http"]

FROM debian:bookworm-slim AS age-of-war-2
COPY --from=builder /app/target/release/age_of_war_2_http .
CMD ["./age_of_war_2_http"]

FROM debian:bookworm-slim AS modern-art-2
COPY --from=builder /app/target/release/modern_art_2_http .
CMD ["./modern_art_2_http"]

FROM debian:bookworm-slim AS cathedral-2
COPY --from=builder /app/target/release/cathedral_2_http .
CMD ["./cathedral_2_http"]

FROM debian:bookworm-slim AS roll-through-the-ages-2
COPY --from=builder /app/target/release/roll_through_the_ages_2_http .
CMD ["./roll_through_the_ages_2_http"]

FROM debian:bookworm-slim AS splendor-2
COPY --from=builder /app/target/release/splendor_2_http .
CMD ["./splendor_2_http"]

FROM debian:bookworm-slim AS texas-holdem-2
COPY --from=builder /app/target/release/texas_holdem_2_http .
CMD ["./texas_holdem_2_http"]
