package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestNextCommandPreserve(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentPreservation] = true
	g.Boards[Mick].Goods[GoodPottery] = 3
	g.Boards[Mick].Food = 3
	g.Phase = PhasePreserve
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.NotEqual(t, PhasePreserve, g.Phase)
}

func TestNextCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	// For reroll
	g.Boards[Mick].Developments[DevelopmentLeadership] = true
	// For invade
	g.Boards[Mick].Developments[DevelopmentSmithing] = true
	g.Boards[Mick].Goods[GoodSpearhead] = 1
	// For trade
	g.Boards[Mick].Developments[DevelopmentShipping] = true
	g.Boards[Mick].Ships = 3
	// Set up for invade
	assert.Equal(t, PhaseRoll, g.Phase)
	g.KeptDice = []Die{
		DiceSkull,
		DiceSkull,
		DiceSkull,
		DiceSkull,
		DiceWorkers,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseExtraRoll, g.Phase)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseInvade, g.Phase)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseBuild, g.Phase)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseTrade, g.Phase)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseBuy, g.Phase)
	_, err = g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, PhaseDiscard, g.Phase)
}
