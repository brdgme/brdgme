package roll_through_the_ages

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

const (
	Mick = iota
	Steve
	BJ
)

func TestGame_KeepSkulls_allDisasterSkip(t *testing.T) {
	g := &Game{}
	g.New(3)
	g.CurrentPlayer = Mick
	g.RolledDice = []int{
		DiceSkull,
		DiceSkull,
		DiceSkull,
	}
	g.KeepSkulls()
	assert.Equal(t, Mick, g.CurrentPlayer)
	assert.Equal(t, PhaseBuy, g.Phase)
}

func TestGame_KeepSkulls_allDisasterLeadership(t *testing.T) {
	g := &Game{}
	g.New(3)
	g.CurrentPlayer = Mick
	g.Boards[Mick].Developments[DevelopmentLeadership] = true
	g.RolledDice = []int{
		DiceSkull,
		DiceSkull,
		DiceSkull,
	}
	g.KeepSkulls()
	assert.Equal(t, Mick, g.CurrentPlayer)
	assert.Equal(t, PhaseExtraRoll, g.Phase)
}
