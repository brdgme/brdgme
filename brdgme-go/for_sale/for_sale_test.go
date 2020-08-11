package for_sale

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
)

var names = []string{"Mick", "Steve", "BJ"}

const (
	Mick = iota
	Steve
	BJ
)

func TestFullGame(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	// Set the state of the game to sorted decks
	_, g.BuildingDeck = BuildingDeck().PopN(2)
	_, g.ChequeDeck = ChequeDeck().PopN(2)
	g.OpenCards, g.BuildingDeck = g.BuildingDeck.PopN(3)
	// Play a round of buying
	assert.Equal(t, []int{Mick}, g.WhoseTurn())
	_, err = g.Command(Mick, "bid 3", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Steve}, g.WhoseTurn())
	_, err = g.Command(Steve, "bid 3", names)
	assert.Error(t, err)
	_, err = g.Command(Steve, "bid 4", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{BJ}, g.WhoseTurn())
	_, err = g.Command(BJ, "pass", names)
	assert.NoError(t, err)
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 17},
		libcard.Card{Rank: 18},
	}, g.OpenCards)
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 16},
	}, g.Hands[2])
	assert.Equal(t, 15, g.Chips[2])
	assert.Equal(t, []int{Mick}, g.WhoseTurn())
	_, err = g.Command(Mick, "pass", names)
	assert.NoError(t, err)
	assert.Equal(t, 14, g.Chips[0])
	assert.Equal(t, 11, g.Chips[1])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 17},
	}, g.Hands[0])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 18},
	}, g.Hands[1])
	assert.Equal(t, []int{Steve}, g.WhoseTurn())
	// One more buying phase so each player has 2 buildings.
	_, err = g.Command(Steve, "pass", names)
	assert.NoError(t, err)
	_, err = g.Command(BJ, "pass", names)
	assert.NoError(t, err)
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 15},
		libcard.Card{Rank: 17},
	}, g.Hands[0])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 13},
		libcard.Card{Rank: 18},
	}, g.Hands[1])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 14},
		libcard.Card{Rank: 16},
	}, g.Hands[2])
	// End the buying phase early and shorten the selling phase.
	g.BuildingDeck = libcard.Deck{}
	_, g.ChequeDeck = g.ChequeDeck.PopN(12)
	g.OpenCards = libcard.Deck{}
	g.StartRound()
	assert.Equal(t, []int{Mick, Steve, BJ}, g.WhoseTurn())
	// Play a round of selling
	_, err = g.Command(BJ, "play 18", names)
	assert.Error(t, err)
	_, err = g.Command(BJ, "play 16", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Mick, Steve}, g.WhoseTurn())
	_, err = g.Command(Steve, "play 18", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Mick}, g.WhoseTurn())
	_, err = g.Command(Mick, "play 17", names)
	assert.NoError(t, err)
	// Because there were only two cards each, assume that the last cards were
	// automatically played.
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 5},
		libcard.Card{Rank: 3},
	}, g.Cheques[0])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 6},
		libcard.Card{Rank: 0},
	}, g.Cheques[1])
	assert.Equal(t, libcard.Deck{
		libcard.Card{Rank: 4},
		libcard.Card{Rank: 0},
	}, g.Cheques[2])
	// Check the game ended
	assert.True(t, g.IsFinished())
	assert.Equal(t, 1, g.Placings()[0])
}
