package zombie_dice

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type RollCommand struct{}

type KeepCommand struct{}

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
	switch output.Value.(type) {
	case RollCommand:
		return g.RollCommand(player, output.Remaining)
	case KeepCommand:
		return g.KeepCommand(player, output.Remaining)
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

func (g *Game) RollCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayerRoll(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) KeepCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Keep(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanRoll(player) {
		parsers = append(parsers, RollParser)
	}
	if g.CanKeep(player) {
		parsers = append(parsers, KeepParser)
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

var RollParser = brdgme.Doc{
	Name: "roll",
	Desc: "push your luck and roll the dice",
	Parser: brdgme.Map{
		Parser: brdgme.Token("roll"),
		Func: func(value interface{}) interface{} {
			return RollCommand{}
		},
	},
}

var KeepParser = brdgme.Doc{
	Name: "keep",
	Desc: "be a coward and keep your brains",
	Parser: brdgme.Map{
		Parser: brdgme.Token("keep"),
		Func: func(value interface{}) interface{} {
			return KeepCommand{}
		},
	},
}
