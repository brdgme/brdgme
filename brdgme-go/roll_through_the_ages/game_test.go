package roll_through_the_ages

import (
	"errors"
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

const (
	Mick = iota
	Steve
	BJ
)

func (g *Game) NewBlank(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 4 {
		return nil, errors.New("Roll Through the Ages is 2-4 player")
	}
	g.Boards = make([]*PlayerBoard, players)
	for i := 0; i < players; i++ {
		g.Boards[i] = NewPlayerBoard()
	}
	g.CurrentPlayer = Mick
	g.Phase = PhaseRoll
	g.RemainingRolls = 2
	return nil, nil
}

var TestPlayers = []string{"Mick", "Steve", "BJ"}

func TestGame_KeepSkulls_allDisasterSkip(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceSkull,
		DiceSkull,
		DiceSkull,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, Mick, g.CurrentPlayer)
	assert.Equal(t, PhaseBuy, g.Phase)
}

func TestGame_KeepSkulls_allDisasterLeadership(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentLeadership] = true
	g.RolledDice = []Die{
		DiceSkull,
		DiceSkull,
		DiceSkull,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, Mick, g.CurrentPlayer)
	assert.Equal(t, PhaseExtraRoll, g.Phase)
}
