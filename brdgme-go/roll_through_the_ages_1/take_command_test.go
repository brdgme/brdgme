package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestTakeCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceWorkers,
		DiceWorkers,
		DiceFoodOrWorkers,
		DiceFoodOrWorkers,
		DiceFoodOrWorkers,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "take w w f", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "build 10 city", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 10, g.Boards[Mick].CityProgress)
}
