package no_thanks

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PassCommand struct{}

type TakeCommand struct{}

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
	case PassCommand:
		return g.PassCommand(player, output.Remaining)
	case TakeCommand:
		return g.TakeCommand(player, output.Remaining)
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

func (g *Game) PassCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Pass(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) TakeCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Take(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanPass(player) {
		parsers = append(parsers, PassParser)
	}
	if g.CanTake(player) {
		parsers = append(parsers, TakeParser)
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

var PassParser = brdgme.Doc{
	Name: "pass",
	Desc: "spend a chip to pass",
	Parser: brdgme.Map{
		Parser: brdgme.Token("pass"),
		Func: func(value interface{}) interface{} {
			return PassCommand{}
		},
	},
}

var TakeParser = brdgme.Doc{
	Name: "take",
	Desc: "take the card and all chips on it",
	Parser: brdgme.Map{
		Parser: brdgme.Token("take"),
		Func: func(value interface{}) interface{} {
			return TakeCommand{}
		},
	},
}
