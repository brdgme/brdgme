package sushizock_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type RollCommand struct {
	Dice []int
}

type StealCommand struct {
	Player int
	Type   TileType
	Num    *int
}

type TakeCommand struct {
	Type TileType
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
	case RollCommand:
		return g.RollCommand(player, value.Dice, output.Remaining)
	case StealCommand:
		return g.StealCommand(player, value.Player, value.Type, value.Num, output.Remaining)
	case TakeCommand:
		return g.TakeCommand(player, value.Type, output.Remaining)
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

func (g *Game) RollCommand(player int, dice []int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.RollDice(player, dice)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) StealCommand(player, enemy int, tileType TileType, num *int, remaining string) (brdgme.CommandResponse, error) {
	var (
		logs []brdgme.Log
		err  error
	)
	if num == nil {
		if tileType == TileTypeBlue {
			logs, err = g.StealBlue(player, enemy)
		} else {
			logs, err = g.StealRed(player, enemy)
		}
	} else {
		if tileType == TileTypeBlue {
			logs, err = g.StealBlueN(player, enemy, *num)
		} else {
			logs, err = g.StealRedN(player, enemy, *num)
		}
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) TakeCommand(player int, tileType TileType, remaining string) (brdgme.CommandResponse, error) {
	var (
		logs []brdgme.Log
		err  error
	)
	if tileType == TileTypeBlue {
		logs, err = g.TakeBlue(player)
	} else {
		logs, err = g.TakeRed(player)
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanRoll(player) {
		parsers = append(parsers, g.RollParser())
	}
	if g.CanSteal(player) {
		parsers = append(parsers, StealParser)
	}
	if g.CanTake(player) {
		parsers = append(parsers, TakeParser)
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) RollParser() brdgme.Parser {
	minI := 1
	maxI := len(g.RolledDice)
	minU := uint(minI)
	maxU := uint(maxI)
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "roll",
				Desc:   "roll dice",
				Parser: brdgme.Token("roll"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "dice",
					Desc: "list of dice numbers to roll, separated by spaces",
					Parser: brdgme.Many{
						Min: &minU,
						Max: &maxU,
						Parser: brdgme.Int{
							Min: &minI,
							Max: &maxI,
						},
						Delim: brdgme.Space{},
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			dice := []int{}
			for _, v := range value.([]interface{})[1].([]interface{}) {
				dice = append(dice, v.(int))
			}
			return RollCommand{Dice: dice}
		},
	}
}

var TileTypeParser = brdgme.Enum{
	Values: []brdgme.EnumValue{
		{Name: "blue", Value: TileTypeBlue},
		{Name: "red", Value: TileTypeRed},
	},
}

var StealParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "steal",
			Desc:   "steal a tile from another player",
			Parser: brdgme.Token("steal"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "opponent",
				Desc:   "the opponent to steal from",
				Parser: brdgme.Player{},
			},
		),
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "color",
				Desc:   "whether to steal red or blue",
				Parser: TileTypeParser,
			},
		),
		brdgme.Opt{
			Parser: brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "tile",
					Desc:   "optional if you have 4 chopsticks, which tile to steal in the stack, 1 for top",
					Parser: brdgme.Int{},
				},
			),
		},
	},
	Func: func(value interface{}) interface{} {
		values := value.([]interface{})
		var num *int
		if values[3] != nil {
			i := values[3].(int)
			num = &i
		}
		return StealCommand{
			Player: values[1].(int),
			Type:   values[2].(TileType),
			Num:    num,
		}
	},
}

var TakeParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "take",
			Desc:   "take a red or blue tile",
			Parser: brdgme.Token("take"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "color",
				Desc:   "whether to take red or blue",
				Parser: TileTypeParser,
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return TakeCommand{
			Type: value.([]interface{})[1].(TileType),
		}
	},
}
