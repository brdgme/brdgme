package greed

import (
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libdie"
)

type ScoreCommand struct {
	Dice []int
}

type RollCommand struct{}

type DoneCommand struct{}

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
	case ScoreCommand:
		return g.ScoreCommand(player, value.Dice, output.Remaining)
	case RollCommand:
		return g.RollCommand(player, output.Remaining)
	case DoneCommand:
		return g.DoneCommand(player, output.Remaining)
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

func (g *Game) ScoreCommand(player int, dice []int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Score(player, dice)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, err
}

func (g *Game) RollCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayerRoll(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) DoneCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Done(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanScore(player) {
		parsers = append(parsers, g.ScoreParser())
	}
	if g.CanRoll(player) {
		parsers = append(parsers, RollParser)
	}
	if g.CanDone(player) {
		parsers = append(parsers, DoneParser)
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func ScoreDiceParser(score Score) brdgme.Parser {
	diceStrs := make([]string, len(score.Dice))
	for i, d := range score.Dice {
		diceStrs[i] = DieNames[d]
	}
	name := strings.Join(diceStrs, "")
	return brdgme.Doc{
		Name: name,
		Desc: fmt.Sprintf("%d points", score.Value),
		Parser: brdgme.Map{
			Parser: brdgme.Token(name),
			Func: func(value interface{}) interface{} {
				return score.Dice
			},
		},
	}
}

func (g *Game) ScoreParser() brdgme.Parser {
	available := []brdgme.Parser{}
	for _, s := range Scores() {
		if ok, _ := libdie.DiceInDice(s.Dice, g.RemainingDice); ok {
			available = append(available, ScoreDiceParser(s))
		}
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "score",
				Desc:   "score dice",
				Parser: brdgme.Token("score"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "dice",
					Desc:   "the dice to score",
					Parser: brdgme.OneOf(available),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return ScoreCommand{
				Dice: value.([]interface{})[1].([]Die),
			}
		},
	}
}

var RollParser = brdgme.Doc{
	Name: "roll",
	Desc: "roll the dice",
	Parser: brdgme.Map{
		Parser: brdgme.Token("roll"),
		Func: func(value interface{}) interface{} {
			return RollCommand{}
		},
	},
}

var DoneParser = brdgme.Doc{
	Name: "done",
	Desc: "finish your turn",
	Parser: brdgme.Map{
		Parser: brdgme.Token("done"),
		Func: func(value interface{}) interface{} {
			return DoneCommand{}
		},
	},
}

func DieParser() brdgme.Parser {
	values := make([]brdgme.EnumValue, len(DieFaces))
	for i, die := range DieFaces {
		values[i] = brdgme.EnumValue{
			Value: die,
			Name:  DieNames[die],
		}
	}
	return brdgme.Enum{
		Values: values,
	}
}
