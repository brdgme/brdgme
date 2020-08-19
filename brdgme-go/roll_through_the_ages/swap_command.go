package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanSwap(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseTrade &&
		g.Boards[player].Developments[DevelopmentShipping] &&
		g.Boards[player].GoodsNum() > 0
}

func (g *Game) Swap(player int, fromGood, toGood Good, amount int) ([]brdgme.Log, error) {
	if !g.CanSwap(player) {
		return nil, errors.New("you can't swap at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be positive")
	}
	if fromGood == toGood {
		return nil, errors.New("you must specify two different goods")
	}
	if amount > g.RemainingShips {
		return nil, fmt.Errorf("you only have %d ships remaining", g.RemainingShips)
	}
	if goodNum := g.Boards[player].Goods[fromGood]; goodNum < amount {
		return nil, fmt.Errorf(
			"you only have %d %s left",
			goodNum,
			GoodStrings[fromGood],
		)
	}
	if max := GoodMaximum(toGood); g.Boards[player].Goods[toGood]+amount > max {
		return nil, fmt.Errorf(
			"the you only have room for %d %s",
			max,
			GoodStrings[toGood],
		)
	}

	g.Boards[player].Goods[fromGood] -= amount
	g.Boards[player].Goods[toGood] += amount
	g.RemainingShips -= amount

	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s swapped {{b}}%d{{/b}} %s for %s`,
		g.RenderName(player),
		amount,
		RenderGoodName(fromGood),
		RenderGoodName(toGood),
	))}

	if g.RemainingShips == 0 {
		logs = append(logs, g.NextPhase()...)
	}
	return logs, nil
}

func (g *Game) SwapCommand(
	player, amount int,
	from, to Good,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Swap(player, from, to, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, nil
}
