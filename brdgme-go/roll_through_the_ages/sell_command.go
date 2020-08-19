package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanSell(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseBuy &&
		g.Boards[player].Developments[DevelopmentGranaries]
}

func (g *Game) SellFood(player, amount int) ([]brdgme.Log, error) {
	if !g.CanSell(player) {
		return nil, errors.New("you can't sell at the moment")
	}
	if amount > g.Boards[player].Food {
		return nil, fmt.Errorf("you only have %d food", g.Boards[player].Food)
	}

	coins := amount * 6
	g.RemainingCoins += coins
	g.Boards[player].Food -= amount
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s sold {{b}}%d{{/b}} %s for {{b}}%d coins{{/b}}`,
		g.RenderName(player),
		amount,
		FoodName,
		coins,
	))}, nil
}

func (g *Game) SellCommand(
	player,
	amount int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.SellFood(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}
