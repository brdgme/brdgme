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
