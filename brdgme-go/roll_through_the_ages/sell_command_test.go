package roll_through_the_ages

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestSellCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentGranaries] = true
	g.Boards[Mick].Food = 10 // Will need to feed 3 cities
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "sell 5", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 2, g.Boards[Mick].Food)
	assert.Equal(t, 30, g.RemainingCoins)
}
