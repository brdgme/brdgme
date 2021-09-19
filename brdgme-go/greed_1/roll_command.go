package greed_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) PlayerRoll(pNum int) ([]brdgme.Log, error) {
	if !g.CanRoll(pNum) {
		return nil, errors.New("can't play at the moment")
	}
	g.TakenThisRoll = false
	if len(g.RemainingDice) > 0 {
		return g.Roll(len(g.RemainingDice)), nil
	}
	return g.Roll(6), nil
}

func (g *Game) CanRoll(player int) bool {
	return player == g.Player && g.TakenThisRoll && !g.IsFinished()
}
