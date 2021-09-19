package love_letter_1

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharKing = Char{
	Name:   "King",
	Number: King,
	Text:   "Trade your hand with another player",
	Color:  render.Blue,
}

func (g *Game) PlayKing(player, target int) ([]brdgme.Log, error) {
	if _, ok := brdgme.IntFind(Countess, g.Hands[player]); ok {
		return nil, errors.New("you must play the Countess")
	}

	if err := g.AssertTarget(player, false, target); err != nil {
		return nil, err
	}

	curRound := g.Round
	logs := g.DiscardCard(player, King)

	if target == player {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s, but had nobody to target so just discarded the card",
			render.Player(player),
			RenderCard(King),
		)))

		if g.Round == curRound {
			// Only go to the next player if the round didn't just end.
			logs = append(logs, g.NextPlayer()...)
		}

		return logs, nil
	}

	logs = append(logs, []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s and swapped hands with %s",
			render.Player(player),
			RenderCard(King),
			render.Player(target),
		)),
		brdgme.NewPrivateLog(fmt.Sprintf(
			"You traded your %s for %s",
			RenderCard(g.Hands[player][0]),
			RenderCard(g.Hands[target][0]),
		), []int{player}),
		brdgme.NewPrivateLog(fmt.Sprintf(
			"You traded your %s for %s",
			RenderCard(g.Hands[target][0]),
			RenderCard(g.Hands[player][0]),
		), []int{target}),
	}...)

	g.Hands[player], g.Hands[target] = g.Hands[target], g.Hands[player]

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
