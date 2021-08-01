# syntax=docker/dockerfile:experimental

FROM rust:1.53.0 AS rust-src
WORKDIR /src
COPY rust .

FROM rust:1.53.0 AS rust-deps
WORKDIR /src
COPY rust .
RUN find . -type f -not \( -name Cargo.lock -or -name Cargo.toml -or -name lib.rs -or -name main.rs \) -delete
RUN cargo build --release

FROM rust-src AS rust-test
RUN cargo test

FROM rust-src AS rust-builder
RUN RUSTFLAGS=-g cargo build --release --out-dir=out -Z unstable-options

FROM rust-builder AS api
WORKDIR /root
COPY --from=rust-builder /src/out/brdgme_api .
CMD ["./brdgme_api"]

FROM beefsack/webify:v1.3.0 AS acquire-1
COPY --from=rust-builder /src/out/acquire_cli /script

FROM beefsack/webify:v1.3.0 AS lost-cities-2
COPY --from=rust-builder /src/out/lost_cities_cli /script

FROM beefsack/webify:v1.3.0 AS lost-cities-1
COPY --from=rust-builder /src/out/lost_cities_1_cli /script
