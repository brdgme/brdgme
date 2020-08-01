FROM rust:1.45.1 AS rust-nightly-2020-07-27
RUN rustup install nightly-2020-07-27

FROM rust-nightly-2020-07-27 AS rust-src
WORKDIR /src
COPY rust .

FROM rust-src AS rust-test
RUN cargo test

FROM rust-src AS rust-builder
RUN RUSTFLAGS=-g cargo build --release

FROM alpine:3.12.0 AS api
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
COPY go.mod .
RUN go build ./...

FROM go-builder AS go-test
RUN go test ./...

FROM go-builder AS age_of_war-builder
RUN go build -o age_of_war brdgme-go/age_of_war/cmd/*.go
RUN pwd

FROM alpine:3.12.0 AS age_of_war
WORKDIR /root
COPY --from=age_of_war-builder /src/age_of_war .
CMD ["./age_of_war"]

FROM node:14.7.0 AS web-src
WORKDIR /src
COPY web .
RUN npm install

FROM web-src AS web-builder
RUN node_modules/.bin/webpack -p

FROM web-src AS web-test
RUN npm test

FROM nginx:1.19.1 AS web
COPY --from=web-builder /src/dist /usr/share/nginx/html

FROM node:14.7.0 AS websocket
EXPOSE 80
WORKDIR /src
COPY websocket .
RUN npm install
RUN node_modules/.bin/tsc
CMD ["node", "dist/index.js"]