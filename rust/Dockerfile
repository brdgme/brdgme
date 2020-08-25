# syntax=docker/dockerfile:experimental

FROM rust:1.45.1 AS rust-nightly-2020-07-27
RUN rustup install nightly-2020-07-27

FROM rust-nightly-2020-07-27 AS rust-src
WORKDIR /src
COPY rust .

FROM rust-src AS rust-test
RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/src/target \
  cargo test

FROM rust-src AS rust-builder
RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/src/target \
  RUSTFLAGS=-g cargo build --release --out-dir=out -Z unstable-options

FROM rust:1.45.1 AS api
WORKDIR /root
COPY --from=rust-builder /src/out/brdgme_api .
CMD ["./brdgme_api"]

FROM beefsack/script-httpd:latest AS acquire-1
COPY --from=rust-builder /src/out/acquire_cli /script

FROM beefsack/script-httpd:latest AS lost-cities-2
COPY --from=rust-builder /src/out/lost_cities_cli /script

FROM beefsack/script-httpd:latest AS lost-cities-1
COPY --from=rust-builder /src/out/lost_cities_1_cli /script