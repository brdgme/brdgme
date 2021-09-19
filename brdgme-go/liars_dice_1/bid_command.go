package liars_dice_1

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) BidCommand(player, quantity, value int, remaining string) (brdgme.CommandResponse, error) {
	if !g.CanBid(player) {
		return brdgme.CommandResponse{}, errors.New("can't bid at the moment")
	}
	if quantity < 1 {
		return brdgme.CommandResponse{}, errors.New("quantity must be a positive number, eg. 5")
	}
	if quantity < g.BidQuantity {
		return brdgme.CommandResponse{}, fmt.Errorf(
			"you can't reduce the quantity of the bid, it is currently at %d",
			g.BidQuantity)
	}
	if value < 1 || value > 6 {
		return brdgme.CommandResponse{}, errors.New("value must be a number between 1 and 6")
	}
	if quantity == g.BidQuantity && value <= g.BidValue {
		return brdgme.CommandResponse{}, errors.New(
			"if you don't increase the bid quantity, you must increase the bid value")
	}
	verb := "increased the bid to"
	if g.BidQuantity == 0 {
		verb = "set the starting bid to"
	}
	g.BidQuantity = quantity
	g.BidValue = value
	g.BidPlayer = g.CurrentPlayer
	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf("%s %s %s",
			render.Player(player), verb,
			RenderBid(g.BidQuantity, g.BidValue))),
	}
	g.CurrentPlayer = g.NextActivePlayer(g.CurrentPlayer)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, nil
}

func (g *Game) CanBid(player int) bool {
	return !g.IsFinished() && g.CurrentPlayer == player
}
