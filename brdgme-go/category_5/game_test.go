package category_5

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

const (
	Mick = iota
	Steve
)

func TestGame_DrawCards(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	g.Discard = g.DrawCards(75)
	assert.Len(t, g.Discard, 75)
	assert.Len(t, g.Deck, 5)
	assert.Len(t, g.DrawCards(10), 10)
	assert.Len(t, g.Discard, 0)
	assert.Len(t, g.Deck, 70)
}

func TestAutoPlayLastCard(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	g.Board = [4][]Card{
		{1},
		{2},
		{3},
		{4},
	}
	g.Hands = map[int][]Card{
		Mick:  {5, 6},
		Steve: {7, 8},
	}
	_, err = g.Command(Mick, "play 5", []string{})
	assert.NoError(t, err)
	_, err = g.Command(Steve, "play 7", []string{})
	assert.NoError(t, err)
	assert.Len(t, g.Hands[Mick], 10)
}

func TestSortCards(t *testing.T) {
	assert.Equal(t, []Card{1, 2, 3}, SortCards([]Card{3, 2, 1}))
}
