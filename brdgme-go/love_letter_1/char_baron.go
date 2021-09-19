package love_letter_1

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharBaron = Char{
	Name:   "Baron",
	Number: Baron,
	Text:   "Compare hands with another player, lowest card is eliminated",
	Color:  render.Green,
}

func (g *Game) PlayBaron(player, target int) ([]brdgme.Log, error) {
	if err := g.AssertTarget(player, false, target); err != nil {
		return nil, err
	}

	curRound := g.Round
	logs := g.DiscardCard(player, Baron)

	if target == player {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s, but had nobody to target so just discarded the card",
			render.Player(player),
			RenderCard(Baron),
		)))

		if g.Round == curRound {
			// Only go to the next player if the round didn't just end.
			logs = append(logs, g.NextPlayer()...)
		}

		return logs, nil
	}

	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s and is comparing hands with %s to see who has a lower card",
		render.Player(player),
		RenderCard(Baron),
		render.Player(target),
	)))
	playerCard := g.Hands[player][0]
	targetCard := g.Hands[target][0]
	logFmt := "You have %s, %s has %s"
	logs = append(logs, brdgme.NewPrivateLog(fmt.Sprintf(
		logFmt,
		RenderCard(playerCard),
		render.Player(target),
		RenderCard(targetCard),
	), []int{player}))
	logs = append(logs, brdgme.NewPrivateLog(fmt.Sprintf(
		logFmt,
		RenderCard(targetCard),
		render.Player(player),
		RenderCard(playerCard),
	), []int{target}))

	eliminate := -1
	diff := Cards[playerCard].Number - Cards[targetCard].Number
	if diff < 0 {
		eliminate = player
		g.Hands[player] = []int{playerCard}
	} else if diff > 0 {
		eliminate = target
		g.Hands[target] = []int{targetCard}
	}

	if eliminate == -1 {
		logs = append(logs, brdgme.NewPublicLog(
			"The cards were equal, nobody is eliminated",
		))
	} else {
		logs = append(logs, g.Eliminate(eliminate)...)
	}

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
