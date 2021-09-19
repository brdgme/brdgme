package liars_dice_1

import (
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	die "github.com/brdgme/brdgme/brdgme-go/libdie"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) CallCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	var (
		resultText   string
		losingPlayer int
	)
	if !g.CanCall(player) {
		return brdgme.CommandResponse{}, errors.New("can't call at the moment")
	}
	quantity := 0
	for _, pd := range g.PlayerDice {
		for _, d := range pd {
			if d == g.BidValue || d == 1 {
				quantity++
			}
		}
	}
	bidPlayerName := render.Player(g.BidPlayer)
	callPlayerName := render.Player(g.CurrentPlayer)
	if quantity < g.BidQuantity {
		// Caller was correct
		losingPlayer = g.BidPlayer
		resultText = fmt.Sprintf("%s bid too high and lost a die",
			bidPlayerName)
	} else {
		// Bidder was correct
		losingPlayer = g.CurrentPlayer
		resultText = fmt.Sprintf("%s bid correctly and %s lost a die",
			bidPlayerName, callPlayerName)
	}
	cells := [][]render.Cell{}
	for _, pNum := range g.ActivePlayers() {
		renderedPlayerDice := []string{}
		for _, d := range g.PlayerDice[pNum] {
			renderedPlayerDie := die.Render(d)
			if d == g.BidValue || d == 1 {
				renderedPlayerDie = render.Fg(render.Red, renderedPlayerDie)
			}
			renderedPlayerDice = append(renderedPlayerDice, renderedPlayerDie)
		}
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(pNum), render.Left),
			render.Cel(render.Bold(strings.Join(renderedPlayerDice, " ")), render.Left),
		})
	}
	g.PlayerDice[losingPlayer] = g.PlayerDice[losingPlayer][1:]
	table := render.Table(cells, 0, 1)
	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(`%s called the bid of %s by %s
Everyone revealed the following dice:
%s
%s`, callPlayerName, RenderBid(g.BidQuantity, g.BidValue), bidPlayerName,
			table, resultText)),
	}
	if !g.IsFinished() {
		g.StartRound()
		g.CurrentPlayer = g.NextActivePlayer(g.CurrentPlayer)
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, nil
}

func (g *Game) CanCall(player int) bool {
	return !g.IsFinished() && g.CurrentPlayer == player && g.BidQuantity != 0
}
