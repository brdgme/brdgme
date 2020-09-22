package love_letter

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharGuard = Char{
	Name:   "Guard",
	Number: Guard,
	Text:   "Guess another player's card to eliminate them, except for Guard",
	Color:  render.Grey,
}

func (g *Game) PlayGuard(player, target, card int) ([]brdgme.Log, error) {
	if err := g.AssertTarget(player, false, target); err != nil {
		return nil, err
	}

	curRound := g.Round
	logs := []brdgme.Log{}

	if target == player {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s, but had nobody to target so just discarded the card",
			render.Player(player),
			RenderCard(Guard),
		)))
		logs = append(logs, g.DiscardCard(player, Guard)...)

		if g.Round == curRound {
			// Only go to the next player if the round didn't just end.
			logs = append(logs, g.NextPlayer()...)
		}

		return logs, nil
	}

	if card == Guard {
		return nil, errors.New("you can't use Guard against other Guards")
	}

	logs = append(logs, g.DiscardCard(player, Guard)...)

	prefix := fmt.Sprintf(
		"%s played %s and guessed that %s is a %s, ",
		render.Player(player),
		RenderCard(Guard),
		render.Player(target),
		RenderCard(card),
	)

	if _, ok := brdgme.IntFind(card, g.Hands[target]); ok {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%sand was correct!",
			prefix,
		)))
		logs = append(logs, g.Eliminate(target)...)
	} else {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%sbut was incorrect",
			prefix,
		)))
	}

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
