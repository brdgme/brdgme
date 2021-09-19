package sushi_go_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanDummy(player int) bool {
	return g.Players == 2 && g.Controller == player &&
		g.Playing[Dummy] == nil
}

func (g *Game) Dummy(player int, card int) ([]brdgme.Log, error) {
	if !g.CanDummy(player) {
		return nil, errors.New("you can't dummy at the moment")
	}

	return g.PlayCards(Dummy, player, []int{card})
}
