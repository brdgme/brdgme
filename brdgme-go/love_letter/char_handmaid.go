package love_letter

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharHandmaid = Char{
	Name:   "Handmaid",
	Number: Handmaid,
	Text:   "Immune to the effects of other players' cards until next turn",
	Color:  render.Black,
}

func (g *Game) PlayHandmaid(player int) ([]brdgme.Log, error) {
	curRound := g.Round
	logs := g.DiscardCard(player, Handmaid)

	g.Protected[player] = true
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s and is immune to the effects of other players' cards until the start of their next turn",
		render.Player(player),
		RenderCard(Handmaid),
	)))

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
