package libpoker

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
)

func buildHandByRanks(ranks []int) libcard.Deck {
	d := libcard.Deck{}
	for _, r := range ranks {
		d = d.Push(libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: r,
		})
	}
	return d
}

func TestCardsBySuit(t *testing.T) {
	hand := libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_ACE,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_8,
		},
	}
	cardsBySuit := CardsBySuit(hand)
	if len(cardsBySuit[libcard.STANDARD_52_SUIT_DIAMONDS]) != 3 {
		t.Fatal("Expected diamonds to be 3")
	}
	rank := cardsBySuit[libcard.STANDARD_52_SUIT_DIAMONDS][0].Rank
	if rank != libcard.STANDARD_52_RANK_ACE {
		t.Fatal("Expected first diamond to be ace, got", rank)
	}
}

func TestIsStraight(t *testing.T) {
	hand := buildHandByRanks([]int{2, 6, 3, 8, 6})
	ok, _ := IsStraight(hand)
	if ok {
		t.Fatal("Detected as straight but isn't")
	}
	hand = buildHandByRanks([]int{2, 6, 3, 4, 5})
	ok, cards := IsStraight(hand)
	if !ok {
		t.Fatal("Didn't detect as straight")
	}
	if len(cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if cards[0].Rank != 6 {
		t.Fatal("Expected high card of 6, got", cards[0].Rank)
	}
	hand = buildHandByRanks([]int{2, 6, 3, 4, 5, 4})
	ok, cards = IsStraight(hand)
	if !ok {
		t.Fatal("Didn't detect as straight")
	}
	if len(cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if cards[0].Rank != 6 {
		t.Fatal("Expected high card of 6, got", cards[0].Rank)
	}
	// Ace as low card
	hand = buildHandByRanks([]int{2, 14, 3, 5, 4})
	ok, cards = IsStraight(hand)
	if !ok {
		t.Fatal("Didn't detect as straight")
	}
	if len(cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if cards[0].Rank != 5 {
		t.Fatal("Expected high card of 5, got", cards[0].Rank)
	}
	// Ace as high card
	hand = buildHandByRanks([]int{11, 10, 13, 12, 14})
	ok, cards = IsStraight(hand)
	if !ok {
		t.Fatal("Didn't detect as straight")
	}
	if len(cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if cards[0].Rank != 14 {
		t.Fatal("Expected high card of 14, got", cards[0].Rank)
	}
}

func TestStraightFlush(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_7,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_STRAIGHT_FLUSH {
		t.Fatal("Expected straight flush, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank != libcard.STANDARD_52_RANK_7 {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[0].Rank)
	}
}

func TestFourOfAKind(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_HEARTS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_FOUR_OF_A_KIND {
		t.Fatal("Expected four of a kind, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank != libcard.STANDARD_52_RANK_3 {
		t.Fatal("Expected first rank of 3, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[4].Rank != libcard.STANDARD_52_RANK_6 {
		t.Fatal("Expected fourth rank of 6, got:",
			handResult.Cards[4].Rank)
	}
}

func TestFullHouse(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_HEARTS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_FULL_HOUSE {
		t.Fatal("Expected full house, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank != libcard.STANDARD_52_RANK_3 {
		t.Fatal("Expected first rank of 3, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[3].Rank != libcard.STANDARD_52_RANK_6 {
		t.Fatal("Expected second rank of 6, got:",
			handResult.Cards[3].Rank)
	}
}

func TestFlush(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_7,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_JACK,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_FLUSH {
		t.Fatal("Expected flush, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank !=
		libcard.STANDARD_52_RANK_JACK {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[1].Rank != libcard.STANDARD_52_RANK_7 {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[1].Rank)
	}
	if handResult.Cards[2].Rank != libcard.STANDARD_52_RANK_5 {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[2].Rank)
	}
	if handResult.Cards[3].Rank != libcard.STANDARD_52_RANK_4 {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[3].Rank)
	}
	if handResult.Cards[4].Rank != libcard.STANDARD_52_RANK_3 {
		t.Fatal("Expected 7 high, got:",
			handResult.Cards[4].Rank)
	}
}

func TestStraight(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_2,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_ACE_HIGH,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_STRAIGHT {
		t.Fatal("Expected straight, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank != libcard.STANDARD_52_RANK_5 {
		t.Fatal("Expected 5 high, got:",
			handResult.Cards[0].Rank)
	}
}

func TestThreeOfAKind(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_2,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_5,
		},
	})
	if handResult.Category != CATEGORY_THREE_OF_A_KIND {
		t.Fatal("Expected three of a kind, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank != libcard.STANDARD_52_RANK_3 {
		t.Fatal("Expected first card to be 3, got:",
			handResult.Cards[0].Rank)
	}
}

func TestTwoPair(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_2,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
	})
	if handResult.Category != CATEGORY_TWO_PAIR {
		t.Fatal("Expected two pair, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank !=
		libcard.STANDARD_52_RANK_KING {
		t.Fatal("Expected first card to be king, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[2].Rank !=
		libcard.STANDARD_52_RANK_3 {
		t.Fatal("Expected third card to be 3, got:",
			handResult.Cards[2].Rank)
	}
	if handResult.Cards[4].Rank !=
		libcard.STANDARD_52_RANK_6 {
		t.Fatal("Expected fifth card to be 6, got:",
			handResult.Cards[4].Rank)
	}
}

func TestOnePair(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_2,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_9,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
	})
	if handResult.Category != CATEGORY_ONE_PAIR {
		t.Fatal("Expected one pair, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank !=
		libcard.STANDARD_52_RANK_KING {
		t.Fatal("Expected first card to be king, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[2].Rank !=
		libcard.STANDARD_52_RANK_9 {
		t.Fatal("Expected third card to be 9, got:",
			handResult.Cards[2].Rank)
	}
	if handResult.Cards[3].Rank !=
		libcard.STANDARD_52_RANK_6 {
		t.Fatal("Expected fourth card to be 6, got:",
			handResult.Cards[3].Rank)
	}
	if handResult.Cards[4].Rank !=
		libcard.STANDARD_52_RANK_4 {
		t.Fatal("Expected fifth card to be 4, got:",
			handResult.Cards[4].Rank)
	}
}

func TestHighCard(t *testing.T) {
	handResult := Result(libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_2,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_3,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_KING,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_6,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_CLUBS,
			Rank: libcard.STANDARD_52_RANK_9,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_QUEEN,
		},
	})
	if handResult.Category != CATEGORY_HIGH_CARD {
		t.Fatal("Expected high card, got:", handResult.Category)
	}
	if len(handResult.Cards) != 5 {
		t.Fatal("Didn't get 5 cards back")
	}
	if handResult.Cards[0].Rank !=
		libcard.STANDARD_52_RANK_KING {
		t.Fatal("Expected first card to be king, got:",
			handResult.Cards[0].Rank)
	}
	if handResult.Cards[1].Rank !=
		libcard.STANDARD_52_RANK_QUEEN {
		t.Fatal("Expected second card to be queen, got:",
			handResult.Cards[1].Rank)
	}
	if handResult.Cards[2].Rank !=
		libcard.STANDARD_52_RANK_9 {
		t.Fatal("Expected third card to be 9, got:",
			handResult.Cards[2].Rank)
	}
	if handResult.Cards[3].Rank !=
		libcard.STANDARD_52_RANK_6 {
		t.Fatal("Expected fourth card to be 6, got:",
			handResult.Cards[3].Rank)
	}
	if handResult.Cards[4].Rank !=
		libcard.STANDARD_52_RANK_4 {
		t.Fatal("Expected fifth card to be 4, got:",
			handResult.Cards[4].Rank)
	}
}

func TestHandScore(t *testing.T) {
	hr := HandResult{
		Category: CATEGORY_STRAIGHT,
		Cards: libcard.Deck{
			libcard.Card{
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Rank: libcard.STANDARD_52_RANK_4,
			},
			libcard.Card{
				Rank: libcard.STANDARD_52_RANK_5,
			},
		},
	}
	hs := hr.HandScore()
	if len(hs) != 4 {
		t.Fatal("Hand score is not length 4")
	}
	if hs[0] != CATEGORY_STRAIGHT {
		t.Fatal("First value isn't straight category")
	}
	if hs[1] != libcard.STANDARD_52_RANK_3 {
		t.Fatal("Second value isn't 3")
	}
	if hs[2] != libcard.STANDARD_52_RANK_4 {
		t.Fatal("Third value isn't 4")
	}
	if hs[3] != libcard.STANDARD_52_RANK_5 {
		t.Fatal("Fourth value isn't 5")
	}
}

func TestWinningHandResult(t *testing.T) {
	handResults := map[int]HandResult{
		// 0 is a pair
		0: Result(libcard.Deck{
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_2,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_SPADES,
				Rank: libcard.STANDARD_52_RANK_KING,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_SPADES,
				Rank: libcard.STANDARD_52_RANK_6,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_4,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_CLUBS,
				Rank: libcard.STANDARD_52_RANK_9,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_KING,
			},
		}),
		// 1 is full house
		1: Result(libcard.Deck{
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_HEARTS,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_SPADES,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_6,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_4,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_CLUBS,
				Rank: libcard.STANDARD_52_RANK_6,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_5,
			},
		}),
		// 2 is same full house
		2: Result(libcard.Deck{
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_HEARTS,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_SPADES,
				Rank: libcard.STANDARD_52_RANK_3,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_6,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_4,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_CLUBS,
				Rank: libcard.STANDARD_52_RANK_6,
			},
			libcard.Card{
				Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
				Rank: libcard.STANDARD_52_RANK_5,
			},
		}),
		3: HandResult{},
	}
	winningResults := WinningHandResult(handResults)
	if len(winningResults) != 2 {
		t.Fatal("There weren't two winners")
	}
	winningResMap := brdgme.IntTally(winningResults)
	if winningResMap[1] != 1 {
		t.Fatal("Hand index 1 wasn't a winner")
	}
	if winningResMap[2] != 1 {
		t.Fatal("Hand index 2 wasn't a winner")
	}
}

// https://github.com/Miniand/brdg.me/issues/4
func TestAceIsInFlushResult(t *testing.T) {
	hand := libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_QUEEN,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_ACE_HIGH,
		},
	}
	communityCards := libcard.Deck{
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_10,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_QUEEN,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_4,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_DIAMONDS,
			Rank: libcard.STANDARD_52_RANK_7,
		},
		libcard.Card{
			Suit: libcard.STANDARD_52_SUIT_SPADES,
			Rank: libcard.STANDARD_52_RANK_4,
		},
	}
	handResult := Result(hand.PushMany(communityCards))
	if len(handResult.Cards) != 5 {
		t.Fatal("There aren't 5 cards in the result")
	}
}
