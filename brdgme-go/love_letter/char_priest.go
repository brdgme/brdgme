package love_letter

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharPriest = Char{
	Name:   "Priest",
	Number: Priest,
	Text:   "Look at another player's hand",
	Color:  render.Cyan,
}

func (g *Game) PlayPriest(player, target int) ([]brdgme.Log, error) {
	if err := g.AssertTarget(player, false, target); err != nil {
		return nil, err
	}

	curRound := g.Round
	logs := g.DiscardCard(player, Priest)

	if target == player {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s, but had nobody to target so just discarded the card",
			render.Player(player),
			RenderCard(Priest),
		)))

		if g.Round == curRound {
			// Only go to the next player if the round didn't just end.
			logs = append(logs, g.NextPlayer()...)
		}

		return logs, nil
	}

	logs = append(logs, []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s and looked at %s's hand",
			render.Player(player),
			RenderCard(Priest),
			render.Player(target),
		)),
		brdgme.NewPrivateLog(fmt.Sprintf(
			"%s has %s",
			render.Player(target),
			brdgme.CommaList(RenderCards(g.Hands[target])),
		), []int{player}),
	}...)

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
