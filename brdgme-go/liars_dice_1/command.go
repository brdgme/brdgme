package liars_dice_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

var (
	MinBidQuantity = 1
	MinBidValue    = 1
	MaxBidValue    = 6
)

type bidCommand struct {
	Quantity, Value int
}

type callCommand struct{}

func (g *Game) Command(player int, input string, players []string) (brdgme.CommandResponse, error) {
	parseOutput, err := g.Parser(player).Parse(input, players)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	switch value := parseOutput.Value.(type) {
	case bidCommand:
		return g.BidCommand(player, value.Quantity, value.Value, parseOutput.Remaining)
	case callCommand:
		return g.CallCommand(player, parseOutput.Remaining)
	}
	return brdgme.CommandResponse{}, errors.New("inexhaustive command handler")
}

func (g *Game) Parser(player int) brdgme.Parser {
	oneOf := brdgme.OneOf{}
	if g.CanBid(player) {
		oneOf = append(oneOf, bidParser)
	}
	if g.CanCall(player) {
		oneOf = append(oneOf, callParser)
	}
	return oneOf
}

var callParser = brdgme.Map{
	Parser: brdgme.Doc{
		Name:   "call",
		Desc:   "call that the bid is too high",
		Parser: brdgme.Token("call"),
	},
	Func: func(value interface{}) interface{} {
		return callCommand{}
	},
}

var bidParser = brdgme.Map{
	Parser: brdgme.Chain([]brdgme.Parser{
		brdgme.Doc{
			Name:   "bid",
			Desc:   "bid the number of dice under all players' cups",
			Parser: brdgme.Token("bid"),
		},
		brdgme.AfterSpace(brdgme.Doc{
			Name: "quantity",
			Desc: "the quantity of dice to bid",
			Parser: brdgme.Int{
				Min: &MinBidQuantity,
			}}),
		brdgme.AfterSpace(brdgme.Doc{
			Name: "value",
			Desc: "the face value of dice to bid, including wild dice (1)",
			Parser: brdgme.Int{
				Min: &MinBidValue,
				Max: &MaxBidValue,
			}}),
	}),
	Func: func(value interface{}) interface{} {
		values := value.([]interface{})
		return bidCommand{
			Quantity: values[1].(int),
			Value:    values[2].(int),
		}
	},
}

func (g *Game) CommandSpec(player int) *brdgme.Spec {
	spec := g.Parser(player).ToSpec()
	return &spec
}
