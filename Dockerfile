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

FROM rust:1.45.1 AS script-httpd
WORKDIR /root
COPY --from=rust-builder /src/out/script_httpd .
CMD ["./script_httpd", "./script"]

FROM script-httpd AS acquire-1
COPY --from=rust-builder /src/out/acquire_cli script

FROM script-httpd AS lost-cities-2
COPY --from=rust-builder /src/out/lost_cities_cli script

FROM script-httpd AS lost-cities-1
COPY --from=rust-builder /src/out/lost_cities_1_cli script

FROM golang:1.14.6 AS go-builder
WORKDIR /src
COPY brdgme-go brdgme-go
COPY go.mod .
RUN go build ./...

FROM go-builder AS go-test
RUN go test ./...

FROM go-builder AS age-of-war-builder
RUN go build -o age_of_war brdgme-go/age_of_war/cmd/*.go

FROM script-httpd AS age-of-war-1
COPY --from=age-of-war-builder /src/age_of_war script

FROM go-builder AS liars-dice-builder
RUN go build -o liars_dice brdgme-go/liars_dice/cmd/*.go

FROM script-httpd AS liars-dice-1
COPY --from=liars-dice-builder /src/liars_dice script

FROM go-builder AS for-sale-builder
RUN go build -o for_sale brdgme-go/for_sale/cmd/*.go

FROM script-httpd AS for-sale-1
COPY --from=for-sale-builder /src/for_sale script

FROM go-builder AS roll-through-the-ages-builder
RUN go build -o roll_through_the_ages brdgme-go/roll_through_the_ages/cmd/*.go

FROM script-httpd AS roll-through-the-ages-1
COPY --from=roll-through-the-ages-builder /src/roll_through_the_ages script

FROM node:14.7.0 AS web-src
WORKDIR /src
COPY web/package.json web/package-lock.json .
RUN npm install
COPY web .

FROM web-src AS web-builder
RUN node_modules/.bin/webpack -p

FROM web-src AS web-test
RUN npm test

FROM nginx:1.19.1 AS web
COPY --from=web-builder /src/dist /usr/share/nginx/html

FROM node:14.7.0 AS websocket
EXPOSE 80
WORKDIR /src
COPY websocket/package.json websocket/package-lock.json .
RUN npm install
COPY websocket .
RUN node_modules/.bin/tsc
CMD ["node", "dist/index.js"]