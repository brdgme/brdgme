package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestSwapCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentShipping] = true
	g.Boards[Mick].Ships = 3
	g.RolledDice = []Die{
		DiceSkull,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
	}
	// Keep dice
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	// Skip build
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	// Swap all wood for spearheads
	_, err = g.Command(Mick, "swap 2 wood spear", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodWood])
	assert.Equal(t, 3, g.Boards[Mick].Goods[GoodSpearhead])
}
