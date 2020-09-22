package love_letter

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var CharPrincess = Char{
	Name:   "Princess",
	Number: Princess,
	Text:   "You are eliminated if you discard the Princess",
	Color:  render.Yellow,
}

func (g *Game) PlayPrincess(player int) ([]brdgme.Log, error) {
	curRound := g.Round

	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s",
		render.Player(player),
		RenderCard(Princess),
	))}
	logs = append(logs, g.DiscardCard(player, Princess)...)

	if g.Round == curRound {
		// Only go to the next player if the round didn't just end.
		logs = append(logs, g.NextPlayer()...)
	}

	return logs, nil
}
