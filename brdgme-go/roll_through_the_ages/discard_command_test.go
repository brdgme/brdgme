package roll_through_the_ages

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestDiscardCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
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
	// Skip buy phase
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseDiscard, g.Phase)
	_, err = g.Command(Mick, "discard 1 wood", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "discard 1 spear", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 1, g.Boards[Mick].Goods[GoodWood])
	assert.Equal(t, 2, g.Boards[Mick].Goods[GoodStone])
	assert.Equal(t, 2, g.Boards[Mick].Goods[GoodPottery])
	assert.Equal(t, 1, g.Boards[Mick].Goods[GoodCloth])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodSpearhead])
}
