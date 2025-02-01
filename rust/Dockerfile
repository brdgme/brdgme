FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY rust .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY rust .
RUN cargo build --release

FROM debian:bookworm-slim AS api
RUN apt-get -y update && apt-get install -y --no-install-recommends libpq5
WORKDIR /root
COPY --from=builder /app/target/release/brdgme_api .
CMD ["./brdgme_api"]

FROM debian:bookworm-slim AS acquire-1
COPY --from=builder /app/target/release/acquire_1_http .
CMD ["./acquire_1_http"]

FROM debian:bookworm-slim AS lost-cities-1
COPY --from=builder /app/target/release/lost_cities_1_http .
CMD ["./lost_cities_1_http"]

FROM debian:bookworm-slim AS lost-cities-2
COPY --from=builder /app/target/release/lost_cities_2_http .
CMD ["./lost_cities_2_http"]
