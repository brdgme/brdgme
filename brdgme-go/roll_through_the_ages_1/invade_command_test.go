package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestInvadeCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentSmithing] = true
	g.RolledDice = []Die{
		DiceSkull,
		DiceSkull,
		DiceSkull,
		DiceSkull,
		DiceGood,
		DiceGood,
	}
	// Keep dice
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseInvade, g.Phase)
	assert.Equal(t, 4, g.Boards[Steve].Disasters)
	assert.Equal(t, 4, g.Boards[BJ].Disasters)
	_, err = g.Command(Mick, "invade 2", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 8, g.Boards[Steve].Disasters)
	assert.Equal(t, 8, g.Boards[BJ].Disasters)
}
