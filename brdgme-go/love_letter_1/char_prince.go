package love_letter_1

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharPrince = Char{
	Name:   "Prince",
	Number: Prince,
	Text:   "Choose a player (or yourself) to discard and draw a new card",
	Color:  render.Purple,
}

func (g *Game) PlayPrince(player, target int) ([]brdgme.Log, error) {
	if _, ok := brdgme.IntFind(Countess, g.Hands[player]); ok {
		return nil, errors.New("you must play the Countess")
	}

	if err := g.AssertTarget(player, true, target); err != nil {
		return nil, err
	}

	curRound := g.Round
	logs := g.DiscardCard(player, Prince)

	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s and made %s discard their hand and draw a new card",
		render.Player(player),
		RenderCard(Prince),
		render.Player(target),
	)))

	logs = append(logs, g.DiscardCardLog(target, g.Hands[target][0])...)
	if g.Round == curRound && !g.Eliminated[target] {
		logs = append(logs, g.DrawCard(target)...)
	}

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
