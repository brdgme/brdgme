package for_sale

import (
	"errors"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type BidCommand struct {
	Amount int
}

type PassCommand struct{}

type PlayCommand struct {
	Building int
}

func (g *Game) Command(
	player int,
	input string,
	players []string,
) (brdgme.CommandResponse, error) {
	parser := g.CommandParser(player)
	if parser == nil {
		return brdgme.CommandResponse{}, errors.New(
			"not expecting any commands at the moment",
		)
	}
	output, err := parser.Parse(input, players)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	switch value := output.Value.(type) {
	case BidCommand:
		return g.BidCommand(player, value.Amount, output.Remaining)
	case PassCommand:
		return g.PassCommand(player, output.Remaining)
	case PlayCommand:
		return g.PlayCommand(player, value.Building, output.Remaining)
	}
	return brdgme.CommandResponse{}, errors.New("inexhaustive command handler")
}

func (g *Game) CommandSpec(player int) *brdgme.Spec {
	parser := g.CommandParser(player)
	if parser != nil {
		spec := parser.ToSpec()
		return &spec
	}
	return nil
}

func (g *Game) BidCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Bid(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, err
}

func (g *Game) PassCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Pass(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) PlayCommand(player, building int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Play(player, building)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanBid(player) {
		parsers = append(parsers, g.BidParser(player))
	}
	if g.CanPass(player) {
		parsers = append(parsers, PassParser)
	}
	if g.CanPlay(player) {
		parsers = append(parsers, g.PlayParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) BidParser(player int) brdgme.Parser {
	max := g.Chips[player]
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "bid",
				Desc:   "bid for a building",
				Parser: brdgme.Token("bid"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "the amount to bid",
					Parser: brdgme.Int{
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return BidCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}

var PassParser = brdgme.Doc{
	Name: "pass",
	Desc: "pass from further bidding",
	Parser: brdgme.Map{
		Parser: brdgme.Token("pass"),
		Func: func(value interface{}) interface{} {
			return PassCommand{}
		},
	},
}

func (g *Game) PlayParser(player int) brdgme.Parser {
	cards := []brdgme.EnumValue{}
	for _, c := range g.Hands[player] {
		value := c.Rank
		cards = append(cards, brdgme.EnumValue{
			Name:  strconv.Itoa(value),
			Value: value,
		})
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "play",
				Desc:   "play a building card",
				Parser: brdgme.Token("play"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "building",
					Desc: "the building card to play",
					Parser: brdgme.Enum{
						Values: cards,
						Exact:  true,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return PlayCommand{
				Building: value.([]interface{})[1].(int),
			}
		},
	}
}
