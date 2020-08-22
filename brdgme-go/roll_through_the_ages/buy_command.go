package roll_through_the_ages

import (
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) BuyCommand(
	player int,
	args BuyCommand,
	remaining string,
) (brdgme.CommandResponse, error) {
	goods := args.Goods.Goods
	if args.Goods.AllGoods {
		for good, num := range g.Boards[player].Goods {
			if num > 0 {
				goods = append(goods, good)
			}
		}
	}

	logs, err := g.BuyDevelopment(player, args.Development, goods)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   g.CurrentPlayer == player,
		Remaining: remaining,
	}, nil
}

func (g *Game) CanBuy(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseBuy
}

func (g *Game) BuyDevelopment(player int, development DevelopmentID, goods []Good) ([]brdgme.Log, error) {
	if !g.CanBuy(player) {
		return nil, errors.New("you can't buy at the moment")
	}
	if g.Boards[player].Developments[development] {
		return nil, errors.New("you already have that development")
	}
	dv, ok := DevelopmentValues[development]
	if !ok {
		return nil, errors.New("invalid development")
	}

	total := g.RemainingCoins
	usedGoods := map[Good]bool{}
	for _, good := range goods {
		if usedGoods[good] {
			continue
		}
		total += GoodValue(good, g.Boards[player].Goods[good])
		usedGoods[good] = true
	}
	if total < dv.Cost {
		return nil, fmt.Errorf(
			`you require %d but your coins and specified goods only amount to %d, you may need to add more goods`,
			dv.Cost,
			total,
		)
	}

	suffix := ""
	if len(usedGoods) > 0 {
		suffixParts := []string{}
		for good := range usedGoods {
			suffixParts = append(suffixParts, RenderGoodName(good))
		}
		suffix = fmt.Sprintf(", using %s", strings.Join(suffixParts, ", "))
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`{{player %d}} bought the {{b}}%s development{{/b}}%s`,
		player,
		dv.Name,
		suffix,
	))}
	g.Boards[player].Developments[development] = true
	for _, good := range goods {
		g.Boards[player].Goods[good] = 0
	}

	logs = append(logs, g.CheckGameEndTriggered(player)...)
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}
