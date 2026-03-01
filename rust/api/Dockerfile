FROM rust:slim-bookworm AS builder
RUN apt-get update && apt-get install -y libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY rust/ .
RUN cargo build --release -p brdgme_api

FROM debian:bookworm-slim AS api
RUN apt-get update && apt-get install -y libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/brdgme_api /usr/local/bin/brdgme_api
ENV ROCKET_ENV=production
EXPOSE 8000
CMD ["/usr/local/bin/brdgme_api"]
