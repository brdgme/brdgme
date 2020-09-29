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

func TestDoneTakesRemainingScoringDice(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	currentPlayer := g.Player
	g.RemainingDice = []Die{DieG, DieG, DieG, DieG, DieR, DieD}
	_, err = g.Command(currentPlayer, "done", []string{})
	assert.NoError(t, err)
	// We should have taken GGG, D, G for 650 points
	assert.Equal(t, 650, g.Scores[currentPlayer], "We should have scored GGG, D and G for 650 points")
}
