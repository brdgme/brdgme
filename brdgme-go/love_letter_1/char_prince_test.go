package love_letter_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestCharPrince_Play_end(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.Hands[Mick] = []int{Prince, Princess}
	g.Hands[Steve] = []int{Prince}
	g.Protected[Steve] = true
	g.Eliminated[BJ] = true
	_, err = g.Command(Mick, "prince mick", names)
	assert.NoError(t, err)
	assert.Equal(t, 2, g.Round)
	assert.Equal(t, 1, g.PlayerPoints[Steve])
	assert.Equal(t, Steve, g.CurrentPlayer)
	assert.Len(t, g.Hands[Mick], 1)
	assert.Len(t, g.Hands[Steve], 2)
	assert.Len(t, g.Hands[BJ], 1)
}
