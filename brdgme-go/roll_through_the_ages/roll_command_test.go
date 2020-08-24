package roll_through_the_ages

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestRollCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceCoins,
		DiceCoins,
		DiceCoins,
		DiceCoins,
		DiceCoins,
		DiceCoins,
		DiceCoins,
	}
	_, err := g.Command(Mick, "roll 2 4 7", TestPlayers)
	assert.NoError(t, err)
}

func TestRollExtraCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentLeadership] = true
	g.RolledDice = []Die{
		DiceCoins,
		DiceCoins,
		DiceCoins,
		DiceCoins,
	}
	g.KeptDice = []Die{
		DiceSkull,
		DiceSkull,
		DiceSkull,
	}
	_, err := g.Command(Mick, "roll 7", TestPlayers)
	assert.Error(t, err)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "roll 7", TestPlayers)
	assert.NoError(t, err)
}
