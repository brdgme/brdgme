package sushi_go

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanPlay(player int) bool {
	return g.Playing[player] == nil
}

func (g *Game) Play(player int, cards []int) ([]brdgme.Log, error) {
	if !g.CanPlay(player) {
		return nil, errors.New("you can't play at the moment")
	}
	l := len(cards)
	if l == 0 || l > 2 {
		return nil, errors.New("you must specify one or two cards to play")
	}
	if l == 2 {
		if _, ok := Contains(CardChopsticks, g.Played[player]); !ok {
			return nil, errors.New("you can only play a second card if you've previously played chopsticks")
		}
		if player == g.Controller && g.Playing[Dummy] == nil &&
			g.Players == 2 && len(g.Hands[player]) == 2 {
			// Need to keep room for the dummy player.
			return nil, errors.New("you can't play two cards now, you have to save one for the dummy player")
		}
	}

	return g.PlayCards(player, player, cards)
}

func (g *Game) PlayCards(toPlayer, fromPlayer int, cards []int) ([]brdgme.Log, error) {
	cardMap := map[int]bool{}
	for _, c := range cards {
		if c < 0 || c >= len(g.Hands[fromPlayer]) {
			return nil, errors.New("that card number is not valid")
		}
		if cardMap[c] {
			return nil, errors.New("please specify different cards")
		}
		if g.Hands[fromPlayer][c] == CardPlayed {
			return nil, errors.New("that card has already been played")
		}
		cardMap[c] = true
	}

	// Valid, do that thing
	g.Playing[toPlayer] = make([]int, len(cards))
	for i, c := range cards {
		g.Playing[toPlayer][i] = g.Hands[fromPlayer][c]
		g.Hands[fromPlayer][c] = CardPlayed
	}

	// Check if everyone has played cards
	for p := 0; p < g.AllPlayers; p++ {
		if g.Playing[p] == nil {
			return nil, nil
		}
	}
	return g.EndHand(), nil
}
