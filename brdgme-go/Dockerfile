FROM golang:1.17.1 AS go-builder
WORKDIR /src
COPY brdgme-go brdgme-go
COPY go.mod .
RUN go build ./...

FROM go-builder AS go-test
RUN go test ./...

FROM go-builder AS age-of-war-1-builder
RUN go build -o age_of_war_1 brdgme-go/age_of_war_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS age-of-war-1
COPY --from=age-of-war-1-builder /src/age_of_war_1 /script

FROM go-builder AS liars-dice-1-builder
RUN go build -o liars_dice_1 brdgme-go/liars_dice_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS liars-dice-1
COPY --from=liars-dice-1-builder /src/liars_dice_1 /script

FROM go-builder AS for-sale-1-builder
RUN go build -o for_sale_1 brdgme-go/for_sale_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS for-sale-1
COPY --from=for-sale-1-builder /src/for_sale_1 /script

FROM go-builder AS roll-through-the-ages-1-builder
RUN go build -o roll_through_the_ages_1 brdgme-go/roll_through_the_ages_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS roll-through-the-ages-1
COPY --from=roll-through-the-ages-1-builder /src/roll_through_the_ages_1 /script

FROM go-builder AS texas-holdem-1-builder
RUN go build -o texas_holdem_1 brdgme-go/texas_holdem_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS texas-holdem-1
COPY --from=texas-holdem-1-builder /src/texas_holdem_1 /script

FROM go-builder AS modern-art-1-builder
RUN go build -o modern_art_1 brdgme-go/modern_art_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS modern-art-1
COPY --from=modern-art-1-builder /src/modern_art_1 /script

FROM go-builder AS no-thanks-1-builder
RUN go build -o no_thanks_1 brdgme-go/no_thanks_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS no-thanks-1
COPY --from=no-thanks-1-builder /src/no_thanks_1 /script

FROM go-builder AS sushizock-1-builder
RUN go build -o sushizock_1 brdgme-go/sushizock_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS sushizock-1
COPY --from=sushizock-1-builder /src/sushizock_1 /script

FROM go-builder AS sushi-go-1-builder
RUN go build -o sushi_go_1 brdgme-go/sushi_go_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS sushi-go-1
COPY --from=sushi-go-1-builder /src/sushi_go_1 /script

FROM go-builder AS zombie-dice-1-builder
RUN go build -o zombie_dice_1 brdgme-go/zombie_dice_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS zombie-dice-1
COPY --from=zombie-dice-1-builder /src/zombie_dice_1 /script

FROM go-builder AS love-letter-1-builder
RUN go build -o love_letter_1 brdgme-go/love_letter_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS love-letter-1
COPY --from=love-letter-1-builder /src/love_letter_1 /script

FROM go-builder AS category-5-1-builder
RUN go build -o category_5_1 brdgme-go/category_5_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS category-5-1
COPY --from=category-5-1-builder /src/category_5_1 /script

FROM go-builder AS cathedral-1-builder
RUN go build -o cathedral_1 brdgme-go/cathedral_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS cathedral-1
COPY --from=cathedral-1-builder /src/cathedral_1 /script

FROM go-builder AS farkle-1-builder
RUN go build -o farkle_1 brdgme-go/farkle_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS farkle-1
COPY --from=farkle-1-builder /src/farkle_1 /script

FROM go-builder AS greed-1-builder
RUN go build -o greed_1 brdgme-go/greed_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS greed-1
COPY --from=greed-1-builder /src/greed_1 /script

FROM go-builder AS splendor-1-builder
RUN go build -o splendor_1 brdgme-go/splendor_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS splendor-1
COPY --from=splendor-1-builder /src/splendor_1 /script

FROM go-builder AS battleship-1-builder
RUN go build -o battleship_1 brdgme-go/battleship_1/cmd/*.go

FROM beefsack/webify:v1.3.0 AS battleship-1
COPY --from=battleship-1-builder /src/battleship_1 /script
