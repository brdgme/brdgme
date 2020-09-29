package greed

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestGame(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	g.RemainingDice = []Die{DieDollar, DieDollar, DieDollar, DieE1, DieE2, DieD}
	_, err = g.Command(g.Player, "score $$$", []string{})
	assert.NoError(t, err)
}
