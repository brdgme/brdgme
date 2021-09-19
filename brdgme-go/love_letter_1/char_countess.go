package love_letter_1

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharCountess = Char{
	Name:   "Countess",
	Number: Countess,
	Text:   "Discard the Countess if you have the King or Prince in your hand",
	Color:  render.Red,
}

func (g *Game) PlayCountess(player int) ([]brdgme.Log, error) {
	curRound := g.Round

	logs := g.DiscardCard(player, Countess)
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s discarded %s, they might have been forced to if they also had %s or %s",
		render.Player(player),
		RenderCard(Countess),
		RenderCard(King),
		RenderCard(Prince),
	)))

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
