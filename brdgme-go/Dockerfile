FROM golang:1.14.6 AS go-builder
WORKDIR /src
COPY brdgme-go brdgme-go
COPY go.mod .
RUN go build ./...

FROM go-builder AS go-test
RUN go test ./...

FROM go-builder AS age-of-war-builder
RUN go build -o age_of_war brdgme-go/age_of_war/cmd/*.go

FROM beefsack/webify:v1.3.0 AS age-of-war-1
COPY --from=age-of-war-builder /src/age_of_war /script

FROM go-builder AS liars-dice-builder
RUN go build -o liars_dice brdgme-go/liars_dice/cmd/*.go

FROM beefsack/webify:v1.3.0 AS liars-dice-1
COPY --from=liars-dice-builder /src/liars_dice /script

FROM go-builder AS for-sale-builder
RUN go build -o for_sale brdgme-go/for_sale/cmd/*.go

FROM beefsack/webify:v1.3.0 AS for-sale-1
COPY --from=for-sale-builder /src/for_sale /script

FROM go-builder AS roll-through-the-ages-builder
RUN go build -o roll_through_the_ages brdgme-go/roll_through_the_ages/cmd/*.go

FROM beefsack/webify:v1.3.0 AS roll-through-the-ages-1
COPY --from=roll-through-the-ages-builder /src/roll_through_the_ages /script

FROM go-builder AS texas-holdem-builder
RUN go build -o texas_holdem brdgme-go/texas_holdem/cmd/*.go

FROM beefsack/webify:v1.3.0 AS texas-holdem-1
COPY --from=texas-holdem-builder /src/texas_holdem /script

FROM go-builder AS modern-art-builder
RUN go build -o modern_art brdgme-go/modern_art/cmd/*.go

FROM beefsack/webify:v1.3.0 AS modern-art-1
COPY --from=modern-art-builder /src/modern_art /script

FROM go-builder AS no-thanks-builder
RUN go build -o no_thanks brdgme-go/no_thanks/cmd/*.go

FROM beefsack/webify:v1.3.0 AS no-thanks-1
COPY --from=no-thanks-builder /src/no_thanks /script

FROM go-builder AS sushizock-builder
RUN go build -o sushizock brdgme-go/sushizock/cmd/*.go

FROM beefsack/webify:v1.3.0 AS sushizock-1
COPY --from=sushizock-builder /src/sushizock /script

FROM go-builder AS sushi-go-builder
RUN go build -o sushi_go brdgme-go/sushi_go/cmd/*.go

FROM beefsack/webify:v1.3.0 AS sushi-go-1
COPY --from=sushi-go-builder /src/sushi_go /script