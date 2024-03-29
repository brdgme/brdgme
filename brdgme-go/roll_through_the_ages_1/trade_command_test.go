package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestTradeCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceFood,
		DiceFood,
		DiceFood,
	}
	g.Boards[Mick].Developments[DevelopmentEngineering] = true
	g.Boards[Mick].Goods[GoodStone] = 3
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "trade 3", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodStone])
	_, err = g.Command(Mick, "build 9 great", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 9, g.Boards[Mick].Monuments[MonumentGreatPyramid])
}
