FROM rust:1.45.1 AS rust-nightly-2020-07-27
RUN rustup install nightly-2020-07-27

FROM rust-nightly-2020-07-27 AS rust-builder
WORKDIR /src
COPY rust .
RUN RUSTFLAGS=-g cargo build --release

FROM alpine:3.12.0 AS brdgme_api
WORKDIR /root
COPY --from=rust-builder /src/target/release/brdgme_api .
CMD ["./brdgme_api"]

FROM alpine:3.12.0 AS acquire
WORKDIR /root
COPY --from=rust-builder /src/target/release/acquire_cli .
CMD ["./acquire_cli"]

FROM alpine:3.12.0 AS lost_cities
WORKDIR /root
COPY --from=rust-builder /src/target/release/lost_cities_cli .
CMD ["./lost_cities_cli"]

FROM golang:1.14.6 AS go-builder
WORKDIR /src
COPY brdgme-go brdgme-go
Copy go.mod .
RUN go build ./...