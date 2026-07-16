FROM golang:1.17.1 AS go-builder
WORKDIR /src
ENV CGO_ENABLED=0
COPY brdgme-go brdgme-go
COPY go.mod .
RUN go build ./...

FROM go-builder AS go-test
RUN go test ./...

FROM go-builder AS age-of-war-1-builder
RUN go build -o age_of_war_1 brdgme-go/age_of_war_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS age-of-war-1
COPY --from=age-of-war-1-builder /src/age_of_war_1 /age_of_war_1
USER 65532
CMD ["/age_of_war_1"]

FROM go-builder AS liars-dice-1-builder
RUN go build -o liars_dice_1 brdgme-go/liars_dice_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS liars-dice-1
COPY --from=liars-dice-1-builder /src/liars_dice_1 /liars_dice_1
USER 65532
CMD ["/liars_dice_1"]

FROM go-builder AS for-sale-1-builder
RUN go build -o for_sale_1 brdgme-go/for_sale_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS for-sale-1
COPY --from=for-sale-1-builder /src/for_sale_1 /for_sale_1
USER 65532
CMD ["/for_sale_1"]

FROM go-builder AS roll-through-the-ages-1-builder
RUN go build -o roll_through_the_ages_1 brdgme-go/roll_through_the_ages_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS roll-through-the-ages-1
COPY --from=roll-through-the-ages-1-builder /src/roll_through_the_ages_1 /roll_through_the_ages_1
USER 65532
CMD ["/roll_through_the_ages_1"]

FROM go-builder AS texas-holdem-1-builder
RUN go build -o texas_holdem_1 brdgme-go/texas_holdem_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS texas-holdem-1
COPY --from=texas-holdem-1-builder /src/texas_holdem_1 /texas_holdem_1
USER 65532
CMD ["/texas_holdem_1"]

FROM go-builder AS modern-art-1-builder
RUN go build -o modern_art_1 brdgme-go/modern_art_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS modern-art-1
COPY --from=modern-art-1-builder /src/modern_art_1 /modern_art_1
USER 65532
CMD ["/modern_art_1"]

FROM go-builder AS no-thanks-1-builder
RUN go build -o no_thanks_1 brdgme-go/no_thanks_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS no-thanks-1
COPY --from=no-thanks-1-builder /src/no_thanks_1 /no_thanks_1
USER 65532
CMD ["/no_thanks_1"]

FROM go-builder AS sushizock-1-builder
RUN go build -o sushizock_1 brdgme-go/sushizock_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS sushizock-1
COPY --from=sushizock-1-builder /src/sushizock_1 /sushizock_1
USER 65532
CMD ["/sushizock_1"]

FROM go-builder AS sushi-go-1-builder
RUN go build -o sushi_go_1 brdgme-go/sushi_go_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS sushi-go-1
COPY --from=sushi-go-1-builder /src/sushi_go_1 /sushi_go_1
USER 65532
CMD ["/sushi_go_1"]

FROM go-builder AS zombie-dice-1-builder
RUN go build -o zombie_dice_1 brdgme-go/zombie_dice_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS zombie-dice-1
COPY --from=zombie-dice-1-builder /src/zombie_dice_1 /zombie_dice_1
USER 65532
CMD ["/zombie_dice_1"]

FROM go-builder AS love-letter-1-builder
RUN go build -o love_letter_1 brdgme-go/love_letter_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS love-letter-1
COPY --from=love-letter-1-builder /src/love_letter_1 /love_letter_1
USER 65532
CMD ["/love_letter_1"]

FROM go-builder AS category-5-1-builder
RUN go build -o category_5_1 brdgme-go/category_5_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS category-5-1
COPY --from=category-5-1-builder /src/category_5_1 /category_5_1
USER 65532
CMD ["/category_5_1"]

FROM go-builder AS cathedral-1-builder
RUN go build -o cathedral_1 brdgme-go/cathedral_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS cathedral-1
COPY --from=cathedral-1-builder /src/cathedral_1 /cathedral_1
USER 65532
CMD ["/cathedral_1"]

FROM go-builder AS farkle-1-builder
RUN go build -o farkle_1 brdgme-go/farkle_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS farkle-1
COPY --from=farkle-1-builder /src/farkle_1 /farkle_1
USER 65532
CMD ["/farkle_1"]

FROM go-builder AS greed-1-builder
RUN go build -o greed_1 brdgme-go/greed_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS greed-1
COPY --from=greed-1-builder /src/greed_1 /greed_1
USER 65532
CMD ["/greed_1"]

FROM go-builder AS splendor-1-builder
RUN go build -o splendor_1 brdgme-go/splendor_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS splendor-1
COPY --from=splendor-1-builder /src/splendor_1 /splendor_1
USER 65532
CMD ["/splendor_1"]

FROM go-builder AS battleship-1-builder
RUN go build -o battleship_1 brdgme-go/battleship_1/cmd/*.go

FROM gcr.io/distroless/static-debian12@sha256:61b7ccecebc7c474a531717de80a94709d20547cdcdaf740c25876f2a8e38b44 AS battleship-1
COPY --from=battleship-1-builder /src/battleship_1 /battleship_1
USER 65532
CMD ["/battleship_1"]
