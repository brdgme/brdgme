package sushi_go

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PlayCommand struct {
	Cards []int
}

type DummyCommand struct {
	Card int
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
	case PlayCommand:
		return g.PlayCommand(player, value.Cards, output.Remaining)
	case DummyCommand:
		return g.DummyCommand(player, value.Card, output.Remaining)
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

func (g *Game) PlayCommand(player int, cards []int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Play(player, cards)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) DummyCommand(player int, card int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Dummy(player, card)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanPlay(player) {
		parsers = append(parsers, g.PlayParser(player))
	}
	if g.CanDummy(player) {
		parsers = append(parsers, g.DummyParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) PlayParser(player int) brdgme.Parser {
	cardsMin := uint(1)
	cardsMax := uint(2)
	cardMin := 1
	cardMax := len(g.Hands[player])
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "play",
				Desc:   "play a card, or two cards if you have previously played chopsticks",
				Parser: brdgme.Token("play"),
			},
			brdgme.AfterSpace(
				brdgme.Many{
					Min: &cardsMin,
					Max: &cardsMax,
					Parser: brdgme.Doc{
						Name: "card",
						Desc: "the card to play",
						Parser: brdgme.Int{
							Min: &cardMin,
							Max: &cardMax,
						},
					},
					Delim: brdgme.Space{},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			rawCards := value.([]interface{})[1].([]interface{})
			cards := make([]int, len(rawCards))
			for i, c := range rawCards {
				cards[i] = c.(int) - 1
			}
			return PlayCommand{
				Cards: cards,
			}
		},
	}
}

func (g *Game) DummyParser(player int) brdgme.Parser {
	cardMin := 1
	cardMax := len(g.Hands[player])
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "dummy",
				Desc:   "play a card for the dummy player",
				Parser: brdgme.Token("dummy"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "card",
					Desc: "the card to play",
					Parser: brdgme.Int{
						Min: &cardMin,
						Max: &cardMax,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return DummyCommand{
				Card: value.([]interface{})[1].(int) - 1,
			}
		},
	}
}
