package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanTrade(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseBuild &&
		g.Boards[player].Developments[DevelopmentEngineering] &&
		g.Boards[player].Goods[GoodStone] > 0
}

func (g *Game) TradeStone(player, amount int) ([]brdgme.Log, error) {
	if !g.CanTrade(player) {
		return nil, errors.New("you can't trade at the moment")
	}
	if stone := g.Boards[player].Goods[GoodStone]; amount > stone {
		return nil, fmt.Errorf("you only have %d stone", stone)
	}

	workers := amount * 3
	g.RemainingWorkers += workers
	g.Boards[player].Goods[GoodStone] -= amount
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s traded {{b}}%d{{_b}} %s for {{b}}%d workers{{_b}}`,
		g.RenderName(player),
		amount,
		RenderGoodName(GoodStone),
		workers,
	))}, nil
}

func (g *Game) TradeCommand(
	player,
	amount int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.TradeStone(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}
