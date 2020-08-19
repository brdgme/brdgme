package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanDiscard(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseDiscard
}

func (g *Game) Discard(player, amount int, good Good) ([]brdgme.Log, error) {
	if !g.CanDiscard(player) {
		return nil, errors.New("you can't discard at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be a positive number")
	}
	if !ContainsGood(good, Goods) {
		return nil, errors.New("invalid good")
	}
	if num := g.Boards[player].Goods[good]; amount > num {
		return nil, fmt.Errorf("you only have %d %s", num, GoodStrings[good])
	}
	goodsOverLimit := g.Boards[player].GoodsOverLimit()
	if amount > goodsOverLimit {
		return nil, fmt.Errorf("you only need to discard %d", goodsOverLimit)
	}
	g.Boards[player].Goods[good] -= amount
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s discarded %d %s",
		g.RenderName(player),
		amount,
		RenderGoodName(good),
	))}
	if g.Boards[player].GoodsOverLimit() <= 0 {
		logs = append(logs, g.NextTurn()...)
	}
	return logs, nil
}

func (g *Game) DiscardCommand(
	player int,
	amount int,
	good Good,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Discard(player, amount, good)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, nil
}
