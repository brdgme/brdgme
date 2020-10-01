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

FROM go-builder AS zombie-dice-builder
RUN go build -o zombie_dice brdgme-go/zombie_dice/cmd/*.go

FROM beefsack/webify:v1.3.0 AS zombie-dice-1
COPY --from=zombie-dice-builder /src/zombie_dice /script

FROM go-builder AS love-letter-builder
RUN go build -o love_letter brdgme-go/love_letter/cmd/*.go

FROM beefsack/webify:v1.3.0 AS love-letter-1
COPY --from=love-letter-builder /src/love_letter /script

FROM go-builder AS category-5-builder
RUN go build -o category_5 brdgme-go/category_5/cmd/*.go

FROM beefsack/webify:v1.3.0 AS category-5-1
COPY --from=category-5-builder /src/category_5 /script

FROM go-builder AS cathedral-builder
RUN go build -o cathedral brdgme-go/cathedral/cmd/*.go

FROM beefsack/webify:v1.3.0 AS cathedral-1
COPY --from=cathedral-builder /src/cathedral /script

FROM go-builder AS farkle-builder
RUN go build -o farkle brdgme-go/farkle/cmd/*.go

FROM beefsack/webify:v1.3.0 AS farkle-1
COPY --from=farkle-builder /src/farkle /script

FROM go-builder AS greed-builder
RUN go build -o greed brdgme-go/greed/cmd/*.go

FROM beefsack/webify:v1.3.0 AS greed-1
COPY --from=greed-builder /src/greed /script

FROM go-builder AS splendor-builder
RUN go build -o splendor brdgme-go/splendor/cmd/*.go

FROM beefsack/webify:v1.3.0 AS splendor-1
COPY --from=splendor-builder /src/splendor /script