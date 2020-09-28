package farkle

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestGame(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	g.RemainingDice = []int{1, 2, 3, 4, 5, 6}
	_, err = g.Command(g.Player, "take 1", []string{})
	assert.NoError(t, err)
}
