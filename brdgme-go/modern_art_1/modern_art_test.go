package modern_art_1

import (
	"encoding/json"
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
)

const (
	MICK = iota
	STEVE
	BJ
	ELVA
)

var playerNames = map[int]string{
	MICK:  "Mick",
	STEVE: "Steve",
	BJ:    "BJ",
	ELVA:  "Elva",
}

func mockGame(t *testing.T) *Game {
	game := &Game{}
	_, err := game.New(4)
	if err != nil {
		t.Fatal(err)
	}
	return game
}

func cloneGame(g *Game) *Game {
	newG := &Game{}
	raw, err := json.Marshal(g)
	if err != nil {
		panic(err)
	}
	err = json.Unmarshal(raw, newG)
	if err != nil {
		panic(err)
	}
	return newG
}

func TestDeck(t *testing.T) {
	// Given a fresh deck
	func() {
		d := Deck()
		// It should have 70 cards
		func() {
			assert.Equal(t, 70, len(d))
		}()
		// It should have 12 Lite Metal cards
		func() {
			i := 0
			for _, c := range d {
				if c.Suit == SUIT_LITE_METAL {
					i += 1
				}
			}
			assert.Equal(t, 12, i)
		}()
		// It should have 13 Yoko cards
		func() {
			i := 0
			for _, c := range d {
				if c.Suit == SUIT_YOKO {
					i += 1
				}
			}
			assert.Equal(t, 13, i)
		}()
		// It should have 14 Christine P cards
		func() {
			i := 0
			for _, c := range d {
				if c.Suit == SUIT_CHRISTINE_P {
					i += 1
				}
			}
			assert.Equal(t, 14, i)
		}()
		// It should have 15 Karl Glitter cards
		func() {
			i := 0
			for _, c := range d {
				if c.Suit == SUIT_KARL_GITTER {
					i += 1
				}
			}
			assert.Equal(t, 15, i)
		}()
		// It should have 16 Krypto cards
		func() {
			i := 0
			for _, c := range d {
				if c.Suit == SUIT_KRYPTO {
					i += 1
				}
			}
			assert.Equal(t, 16, i)
		}()
	}()
}

func TestStart(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// It should have given each player 9 cards for 4 players
		func() {
			assert.Equal(t, 9, len(g.PlayerHands[0]))
			assert.Equal(t, 9, len(g.PlayerHands[1]))
			assert.Equal(t, 9, len(g.PlayerHands[2]))
			assert.Equal(t, 9, len(g.PlayerHands[3]))
		}()
		// It should have left 34 cards in the deck
		func() {
			assert.Equal(t, 34, len(g.Deck))
		}()
		// It should have given $100 to each player
		func() {
			assert.Equal(t, 100, g.PlayerMoney[0])
			assert.Equal(t, 100, g.PlayerMoney[1])
			assert.Equal(t, 100, g.PlayerMoney[2])
			assert.Equal(t, 100, g.PlayerMoney[3])
		}()
	}()
}

func TestOpenAuction(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given BJ has a Lite Metal Open Auction card
		func() {
			g := cloneGame(g)
			g.CurrentPlayer = BJ
			g.PlayerHands[BJ] = g.PlayerHands[BJ].Push(libcard.Card{
				SUIT_LITE_METAL, RANK_OPEN})
			// Given BJ plays the Lite Metal Open Auction card
			func() {
				g := cloneGame(g)
				_, err := g.Command(BJ, "play lmop", []string{})
				assert.NoError(t, err)
				assert.Equal(t, STATE_AUCTION, g.State)
				assert.Equal(t, 1, len(g.CurrentlyAuctioning))
				// Given Steve bids
				func() {
					g := cloneGame(g)
					_, err := g.Command(STEVE, "bid 10", []string{})
					assert.NoError(t, err)
					assert.Equal(t, STATE_AUCTION, g.State)
					// Given the other players all pass
					func() {
						g := cloneGame(g)
						_, err := g.Command(MICK, "pass", []string{})
						assert.NoError(t, err)
						_, err = g.Command(BJ, "pass", []string{})
						assert.NoError(t, err)
						_, err = g.Command(ELVA, "pass", []string{})
						assert.NoError(t, err)
						// It should give the card to Steve and go to the next player
						func() {
							assert.Equal(t, STATE_PLAY_CARD, g.State)
							assert.Equal(t, ELVA, g.CurrentPlayer)
							assert.Equal(t, 1, len(g.PlayerPurchases[STEVE]))
							assert.Equal(t, 90, g.PlayerMoney[STEVE])
							assert.Equal(t, 110, g.PlayerMoney[BJ])
						}()
					}()
				}()
				// Given nobody bids
				func() {
					g := cloneGame(g)
					_, err := g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(STEVE, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(ELVA, "pass", []string{})
					assert.NoError(t, err)
					// It should give BJ the card for nothing
					func() {
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, ELVA, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[BJ]))
						assert.Equal(t, 100, g.PlayerMoney[BJ])
					}()
				}()
			}()
		}()
	}()
}

func TestFixedPriceAuction(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given Elva has a Christine P Fixed Price Auction card
		func() {
			g := cloneGame(g)
			g.CurrentPlayer = ELVA
			g.PlayerHands[ELVA] = g.PlayerHands[ELVA].Push(libcard.Card{
				SUIT_CHRISTINE_P, RANK_FIXED_PRICE})
			// Given Elva plays the Christine P Fixed Price Auction card and sets the price at 15
			func() {
				g := cloneGame(g)
				_, err := g.Command(ELVA, "play cpfp", []string{})
				assert.NoError(t, err)
				assert.Equal(t, STATE_AUCTION, g.State)
				assert.Equal(t, 1, len(g.CurrentlyAuctioning))
				_, err = g.Command(ELVA, "price 15", []string{})
				assert.NoError(t, err)
				// Given Mick passes and Steve buys
				func() {
					g := cloneGame(g)
					_, err := g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					assert.Equal(t, STATE_AUCTION, g.State)
					_, err = g.Command(STEVE, "buy", []string{})
					assert.NoError(t, err)
					// Steve should receive the card for the given price
					func() {
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, MICK, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[STEVE]))
						assert.Equal(t, 85, g.PlayerMoney[STEVE])
						assert.Equal(t, 115, g.PlayerMoney[ELVA])
					}()
				}()
				// Given nobody bids
				func() {
					g := cloneGame(g)
					_, err := g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(STEVE, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(BJ, "pass", []string{})
					assert.NoError(t, err)
					// It should give the card to Elva for the given price
					func() {
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, MICK, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[ELVA]))
						assert.Equal(t, 85, g.PlayerMoney[ELVA])
					}()
				}()
			}()
		}()
	}()
}

func TestSealedAuction(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given Elva has a Krypto Sealed Auction card
		func() {
			g := cloneGame(g)
			g.CurrentPlayer = ELVA
			g.PlayerHands[ELVA] = g.PlayerHands[ELVA].Push(libcard.Card{
				SUIT_KRYPTO, RANK_SEALED})
			// Given Elva plays the Krypto Sealed Auction card
			func() {
				g := cloneGame(g)
				_, err := g.Command(ELVA, "play krsl", []string{})
				assert.NoError(t, err)
				assert.Equal(t, STATE_AUCTION, g.State)
				assert.Equal(t, 1, len(g.CurrentlyAuctioning))
				// Given everyone bids different amounts
				func() {
					g := cloneGame(g)
					_, err := g.Command(MICK, "bid 4", []string{})
					assert.NoError(t, err)
					_, err = g.Command(STEVE, "bid 5", []string{})
					assert.NoError(t, err)
					_, err = g.Command(BJ, "bid 3", []string{})
					assert.NoError(t, err)
					_, err = g.Command(ELVA, "bid 1", []string{})
					assert.NoError(t, err)
					// Steve should receive the card for the given price
					func() {
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, MICK, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[STEVE]))
						assert.Equal(t, 95, g.PlayerMoney[STEVE])
						assert.Equal(t, 105, g.PlayerMoney[ELVA])
					}()
				}()
				// Given nobody bids
				func() {
					g := cloneGame(g)
					_, err := g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(STEVE, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(ELVA, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(BJ, "pass", []string{})
					assert.NoError(t, err)
					// It should give the card to Elva for free
					func() {
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, MICK, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[ELVA]))
						assert.Equal(t, 100, g.PlayerMoney[ELVA])
					}()
				}()
			}()
		}()
	}()
}

func TestDoubleAuction(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given Elva has a Karl Glitter Double Auction card and Steve has a Karl Glitter Sealed Auction card
		func() {
			g := cloneGame(g)
			g.CurrentPlayer = ELVA
			g.PlayerHands[ELVA] = g.PlayerHands[ELVA].Push(libcard.Card{
				SUIT_KARL_GITTER, RANK_DOUBLE}).Push(libcard.Card{
				SUIT_KARL_GITTER, RANK_SEALED})
			g.PlayerHands[STEVE] = g.PlayerHands[STEVE].Push(libcard.Card{
				SUIT_KARL_GITTER, RANK_SEALED})
			// Given Elva plays the Karl Glitter Double Auction card
			func() {
				g := cloneGame(g)
				_, err := g.Command(ELVA, "play kgdb", []string{})
				assert.NoError(t, err)
				assert.Equal(t, STATE_AUCTION, g.State)
				assert.Equal(t, 1, len(g.CurrentlyAuctioning))
				// Given Elva passes, Mick passes and Steve plays his KG Sealed
				func() {
					g := cloneGame(g)
					_, err := g.Command(ELVA, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(STEVE, "add kgsl", []string{})
					assert.NoError(t, err)
					// It should start a new sealed auction with Steve as the auctioneer
					func() {
						g := cloneGame(g)
						assert.Equal(t, STEVE, g.CurrentPlayer)
						assert.Equal(t, 2, len(g.CurrentlyAuctioning))
						assert.True(t, g.IsAuction())
						assert.Equal(t, RANK_SEALED, g.AuctionType())
						// Given everyone bids different amounts
						func() {
							g := cloneGame(g)
							_, err := g.Command(MICK, "bid 8", []string{})
							assert.NoError(t, err)
							_, err = g.Command(STEVE, "bid 5", []string{})
							assert.NoError(t, err)
							_, err = g.Command(BJ, "bid 3", []string{})
							assert.NoError(t, err)
							_, err = g.Command(ELVA, "bid 1", []string{})
							assert.NoError(t, err)
							// Mick should receive both the cards for the given price
							func() {
								assert.Equal(t, STATE_PLAY_CARD, g.State)
								assert.Equal(t, BJ, g.CurrentPlayer)
								assert.Equal(t, 2, len(g.PlayerPurchases[MICK]))
								assert.Equal(t, 92, g.PlayerMoney[MICK])
								assert.Equal(t, 108, g.PlayerMoney[STEVE])
								assert.Equal(t, 100, g.PlayerMoney[ELVA])
							}()
						}()
					}()
				}()
			}()
		}()
	}()
}

func TestDoubleAuctionEndsRound(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.CurrentPlayer = MICK
	g.PlayerPurchases = map[int]libcard.Deck{
		MICK: libcard.Deck{
			libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		},
		STEVE: libcard.Deck{
			libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		},
		BJ: libcard.Deck{
			libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		},
	}
	g.PlayerHands[MICK] = libcard.Deck{
		libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
		libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE},
	}
	g.PlayerHands[STEVE] = libcard.Deck{
		libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
		libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
		libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
		libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
	}
	_, err = g.Command(MICK, "play lmdb", []string{})
	assert.NoError(t, err)
	_, err = g.Command(MICK, "pass", []string{})
	assert.NoError(t, err)
	_, err = g.Command(STEVE, "add lmop", []string{})
	assert.NoError(t, err)
	assert.Equal(t, 1, g.Round)
	assert.Equal(t, BJ, g.CurrentPlayer)
}

func TestOnceAroundAuction(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given Mick has a Yoko Once Around Auction card
		func() {
			g := cloneGame(g)
			g.CurrentPlayer = MICK
			g.PlayerHands[MICK] = g.PlayerHands[MICK].Push(libcard.Card{
				SUIT_YOKO, RANK_ONCE_AROUND})
			// Given Mick plays the Yoko Once Around Auction card
			func() {
				g := cloneGame(g)
				_, err := g.Command(MICK, "play yooa", []string{})
				assert.NoError(t, err)
				assert.Equal(t, STATE_AUCTION, g.State)
				assert.Equal(t, 1, len(g.CurrentlyAuctioning))
				// Given some bids
				func() {
					g := cloneGame(g)
					_, err := g.Command(STEVE, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(BJ, "bid 5", []string{})
					assert.NoError(t, err)
					_, err = g.Command(ELVA, "bid 7", []string{})
					assert.NoError(t, err)
					_, err = g.Command(MICK, "pass", []string{})
					assert.NoError(t, err)
					// It should give the card to Elva
					func() {
						g := cloneGame(g)
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, STEVE, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[ELVA]))
						assert.Equal(t, 107, g.PlayerMoney[MICK])
						assert.Equal(t, 93, g.PlayerMoney[ELVA])
					}()
				}()
				// Given everyone passes
				func() {
					g := cloneGame(g)
					_, err := g.Command(STEVE, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(BJ, "pass", []string{})
					assert.NoError(t, err)
					_, err = g.Command(ELVA, "pass", []string{})
					assert.NoError(t, err)
					// It should give the card to Mick for free
					func() {
						g := cloneGame(g)
						assert.Equal(t, STATE_PLAY_CARD, g.State)
						assert.Equal(t, STEVE, g.CurrentPlayer)
						assert.Equal(t, 1, len(g.PlayerPurchases[MICK]))
						assert.Equal(t, 100, g.PlayerMoney[MICK])
					}()
				}()
			}()
		}()
	}()
}

func TestEndOfRound(t *testing.T) {
	// Given a new game
	func() {
		g := mockGame(t)
		// Given there are already 3 Lite Metal on the board
		func() {
			g := cloneGame(g)
			g.PlayerPurchases[MICK] = libcard.Deck{
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
			}
			g.PlayerPurchases[STEVE] = libcard.Deck{
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
			}
			// Given Mick plays a Lite Metal Double Auction
			func() {
				g := cloneGame(g)
				g.PlayerHands[MICK] = g.PlayerHands[MICK].Push(
					libcard.Card{SUIT_LITE_METAL, RANK_DOUBLE})
				_, err := g.Command(MICK, "play lmdb", []string{})
				assert.NoError(t, err)
				// It should be the same round
				func() {
					g := cloneGame(g)
					assert.Equal(t, 0, g.Round)
				}()
				// Given Mick adds another Lite Metal
				func() {
					g := cloneGame(g)
					g.PlayerHands[MICK] = g.PlayerHands[MICK].Push(
						libcard.Card{SUIT_LITE_METAL, RANK_OPEN})
					_, err := g.Command(MICK, "add lmop", []string{})
					assert.NoError(t, err)
					// It should be the next round and values should be added to artists
					func() {
						g := cloneGame(g)
						assert.Equal(t, 1, g.Round)
						assert.Equal(t, 30, g.SuitValue(SUIT_LITE_METAL))
						assert.Equal(t, 160, g.PlayerMoney[MICK])
						assert.Equal(t, 130, g.PlayerMoney[STEVE])
						assert.Equal(t, 100, g.PlayerMoney[BJ])
						assert.Equal(t, 100, g.PlayerMoney[ELVA])
					}()
				}()
			}()
		}()
		// Given there are already 4 Lite Metal on the board
		func() {
			g := cloneGame(g)
			g.PlayerPurchases[MICK] = libcard.Deck{
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
			}
			g.PlayerPurchases[STEVE] = libcard.Deck{
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
			}
			g.PlayerPurchases[BJ] = libcard.Deck{
				libcard.Card{SUIT_LITE_METAL, RANK_OPEN},
			}
			// Given Mick adds another Lite Metal
			func() {
				g := cloneGame(g)
				g.PlayerHands[MICK] = g.PlayerHands[MICK].Push(
					libcard.Card{SUIT_LITE_METAL, RANK_OPEN})
				_, err := g.Command(MICK, "play lmop", []string{})
				assert.NoError(t, err)
				// It should be the next round and values should be added to artists
				func() {
					g := cloneGame(g)
					assert.Equal(t, 1, g.Round)
					assert.Equal(t, 30, g.SuitValue(SUIT_LITE_METAL))
					assert.Equal(t, 160, g.PlayerMoney[MICK])
					assert.Equal(t, 130, g.PlayerMoney[STEVE])
					assert.Equal(t, 130, g.PlayerMoney[BJ])
					assert.Equal(t, 100, g.PlayerMoney[ELVA])
				}()
			}()
			// Given it is the final round
			func() {
				g := cloneGame(g)
				g.Round = 3
				// Given Mick adds another Lite Metal
				func() {
					g := cloneGame(g)
					g.PlayerHands[MICK] = g.PlayerHands[MICK].Push(
						libcard.Card{SUIT_LITE_METAL, RANK_OPEN})
					_, err := g.Command(MICK, "play lmop", []string{})
					assert.NoError(t, err)
					// It should be the end of the game and values should be added to artists
					func() {
						g := cloneGame(g)
						assert.True(t, g.IsFinished())
						assert.Equal(t, 30, g.SuitValue(SUIT_LITE_METAL))
						assert.Equal(t, 160, g.PlayerMoney[MICK])
						assert.Equal(t, 130, g.PlayerMoney[STEVE])
						assert.Equal(t, 130, g.PlayerMoney[BJ])
						assert.Equal(t, 100, g.PlayerMoney[ELVA])
						placings := g.Placings()
						assert.Equal(t, 1, placings[MICK])
					}()
				}()
			}()
		}()
	}()
}
