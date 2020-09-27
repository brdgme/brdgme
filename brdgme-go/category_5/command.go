package category_5

import (
	"errors"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PlayCommand struct {
	Card Card
}

type ChooseCommand struct {
	Row int
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
	case ChooseCommand:
		return g.ChooseCommand(player, value.Row, output.Remaining)
	case PlayCommand:
		return g.PlayCommand(player, value.Card, output.Remaining)
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

func (g *Game) ChooseCommand(player, row int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Choose(player, row)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) PlayCommand(player int, card Card, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Play(player, card)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanChoose(player) {
		parsers = append(parsers, g.ChooseParser())
	}
	if g.CanPlay(player) {
		parsers = append(parsers, g.PlayParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) ChooseParser() brdgme.Parser {
	min := 1
	max := len(g.Board)
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "choose",
				Desc:   "choose the row to take",
				Parser: brdgme.Token("choose"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "row",
					Desc: "the row to take",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return ChooseCommand{
				Row: value.([]interface{})[1].(int),
			}
		},
	}
}

func (g *Game) PlayParser(player int) brdgme.Parser {
	cards := []brdgme.EnumValue{}
	for _, c := range g.Hands[player] {
		cards = append(cards, brdgme.EnumValue{
			Name:  strconv.Itoa(int(c)),
			Value: c,
		})
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "play",
				Desc:   "play a card",
				Parser: brdgme.Token("play"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "card",
					Desc: "the card to play",
					Parser: brdgme.Enum{
						Values: cards,
						Exact:  true,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return PlayCommand{
				Card: value.([]interface{})[1].(Card),
			}
		},
	}
}
