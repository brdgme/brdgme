package texas_holdem_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type AllInCommand struct{}

type CallCommand struct{}

type CheckCommand struct{}

type FoldCommand struct{}

type RaiseCommand struct {
	Amount int
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
	case AllInCommand:
		return g.AllInCommand(player, output.Remaining)
	case CallCommand:
		return g.CallCommand(player, output.Remaining)
	case CheckCommand:
		return g.CheckCommand(player, output.Remaining)
	case FoldCommand:
		return g.FoldCommand(player, output.Remaining)
	case RaiseCommand:
		return g.RaiseCommand(player, value.Amount, output.Remaining)
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

func (g *Game) AllInCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.AllIn(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CallCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Call(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CheckCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Check(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) FoldCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Fold(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) RaiseCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Raise(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanAllIn(player) {
		parsers = append(parsers, AllInParser)
	}
	if g.CanCall(player) {
		parsers = append(parsers, CallParser)
	}
	if g.CanCheck(player) {
		parsers = append(parsers, CheckParser)
	}
	if g.CanFold(player) {
		parsers = append(parsers, FoldParser)
	}
	if g.CanRaise(player) {
		parsers = append(parsers, g.RaiseParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

var AllInParser = brdgme.Doc{
	Name: "allin",
	Desc: "bet all your money and go all in",
	Parser: brdgme.Map{
		Parser: brdgme.Token("allin"),
		Func: func(value interface{}) interface{} {
			return AllInCommand{}
		},
	},
}

var CallParser = brdgme.Doc{
	Name: "call",
	Desc: "increase your bet to match the current bet",
	Parser: brdgme.Map{
		Parser: brdgme.Token("call"),
		Func: func(value interface{}) interface{} {
			return CallCommand{}
		},
	},
}

var CheckParser = brdgme.Doc{
	Name: "check",
	Desc: "continue without betting more money",
	Parser: brdgme.Map{
		Parser: brdgme.Token("check"),
		Func: func(value interface{}) interface{} {
			return CheckCommand{}
		},
	},
}

var FoldParser = brdgme.Doc{
	Name: "fold",
	Desc: "forfeit this hand",
	Parser: brdgme.Map{
		Parser: brdgme.Token("fold"),
		Func: func(value interface{}) interface{} {
			return FoldCommand{}
		},
	},
}

func (g *Game) RaiseParser(player int) brdgme.Parser {
	behindCurrentBet := g.CurrentBet() - g.Bets[player]
	min := g.MinRaise()
	max := g.PlayerMoney[player] - behindCurrentBet
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "raise",
				Desc:   "bet higher than the highest bet by this amount",
				Parser: brdgme.Token("raise"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "the amount to raise above the highest bet",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return RaiseCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}
