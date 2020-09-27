package category_5

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanPlay(player int) bool {
	return !g.Resolving && g.Plays[player] == 0
}

func (g *Game) Play(player int, card Card) ([]brdgme.Log, error) {
	if !g.CanPlay(player) {
		return nil, errors.New("you can't play at the moment")
	}

	var ok bool
	g.Hands[player], ok = RemoveCard(g.Hands[player], card)
	if !ok {
		return nil, errors.New("you don't have that card")
	}

	g.Plays[player] = card

	// Check if everyone had played
	for p := 0; p < g.Players; p++ {
		if g.Plays[p] == 0 {
			// Some people haven't played yet
			return nil, nil
		}
	}
	return g.ResolvePlays(), nil
}
