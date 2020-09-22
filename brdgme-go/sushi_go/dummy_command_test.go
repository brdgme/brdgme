package sushi_go

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestDummyCommand_Call(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	assert.Equal(t, []int{Mick, Steve}, g.WhoseTurn())

	// Mick plays a card
	mickCard := g.Hands[Mick][0]
	_, err = g.Command(Mick, "play 1", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{mickCard}, g.Playing[Mick])
	// Mick hasn't played the dummy card yet so should still be their turn.
	assert.Equal(t, []int{Mick, Steve}, g.WhoseTurn())

	// Steve plays a card
	steveCard := g.Hands[Steve][8]
	_, err = g.Command(Steve, "play 9", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Mick}, g.WhoseTurn())

	// Mick plays the dummy card
	dummyCard := g.Hands[Mick][4]
	_, err = g.Command(Mick, "dummy 5", names)
	assert.NoError(t, err)
	// Plays should have happened now.
	assert.Equal(t, []int{mickCard}, g.Played[Mick])
	assert.Equal(t, []int{steveCard}, g.Played[Steve])
	assert.Equal(t, []int{dummyCard}, g.Played[Dummy])
	assert.Equal(t, []int{Mick, Steve}, g.WhoseTurn())
}
