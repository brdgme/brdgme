package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) TradeCommand(
	player int,
	args TradeCommand,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.TradeCommand(player, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}
}

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
	return []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			`{{player %d}} traded {{b}}%d{{/b}} %s for {{b}}%d workers{{/b}}`,
			player,
			amount,
			RenderGoodName(GoodStone),
			workers,
		)),
	}, nil
}
