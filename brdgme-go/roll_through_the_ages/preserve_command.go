package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanPreserve(player int) bool {
	b := g.Boards[player]
	return g.CurrentPlayer == player && g.Phase == PhasePreserve &&
		b.Developments[DevelopmentPreservation] && b.Goods[GoodPottery] > 0 &&
		b.Food > 0
}

func (g *Game) Preserve(player int) ([]brdgme.Log, error) {
	if !g.CanPreserve(player) {
		return nil, errors.New("you can't preserve at the moment")
	}

	g.Boards[player].Food *= 2
	g.Boards[player].Goods[GoodPottery] -= 1

	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s used {{b}}preservation{{/b}} to double their food to {{b}}%d{{/b}} for {{b}}1 pottery{{/b}}`,
		g.RenderName(player),
		g.Boards[player].Food,
	))}
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}

func (g *Game) PreserveCommand(
	player int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Preserve(player)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   g.CurrentPlayer == player,
		Remaining: remaining,
	}, err
}
