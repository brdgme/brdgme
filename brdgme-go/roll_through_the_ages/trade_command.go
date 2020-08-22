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
		`%s traded {{b}}%d{{/b}} %s for {{b}}%d workers{{/b}}`,
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
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   g.CurrentPlayer == player,
		Remaining: remaining,
	}, err
}
