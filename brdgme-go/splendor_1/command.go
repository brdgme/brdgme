package splendor_1

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type BuyCommand struct {
	Row, Col int
}

type DiscardCommand struct {
	Tokens []int
}

type ReserveCommand struct {
	Row, Col int
}

type TakeCommand struct {
	Tokens []int
}

type VisitCommand struct {
	Noble int
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
	case BuyCommand:
		return g.BuyCommand(player, value.Row, value.Col, output.Remaining)
	case DiscardCommand:
		return g.DiscardCommand(player, value.Tokens, output.Remaining)
	case ReserveCommand:
		return g.ReserveCommand(player, value.Row, value.Col, output.Remaining)
	case TakeCommand:
		return g.TakeCommand(player, value.Tokens, output.Remaining)
	case VisitCommand:
		return g.VisitCommand(player, value.Noble, output.Remaining)
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

func (g *Game) BuyCommand(player, row, col int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Buy(player, row, col)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) DiscardCommand(player int, tokens []int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Discard(player, tokens)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) ReserveCommand(player, row, col int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Reserve(player, row, col)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) TakeCommand(player int, tokens []int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Take(player, tokens)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) VisitCommand(player, noble int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Visit(player, noble)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanBuy(player) {
		parsers = append(parsers, g.BuyParser(player))
	}
	if g.CanDiscard(player) {
		parsers = append(parsers, g.DiscardParser(player))
	}
	if g.CanReserve(player) {
		parsers = append(parsers, g.ReserveParser(player))
	}
	if g.CanTake(player) {
		parsers = append(parsers, g.TakeParser(player))
	}
	if g.CanVisit(player) {
		parsers = append(parsers, g.VisitParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

type ParsedLoc struct {
	Row, Col int
}

func (g *Game) LocParser(player int) brdgme.Parser {
	values := []brdgme.EnumValue{}
	for row, cards := range g.Board {
		for col := range cards {
			values = append(values, brdgme.EnumValue{
				Name: fmt.Sprintf("%c%d", 'A'+col, row+1),
				Value: ParsedLoc{
					Row: row,
					Col: col,
				},
			})
		}
	}
	for col := range g.PlayerBoards[player].Reserve {
		values = append(values, brdgme.EnumValue{
			Name: fmt.Sprintf("%c4", 'A'+col),
			Value: ParsedLoc{
				Row: 3,
				Col: col,
			},
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}

func TokenParser(includeGold bool) brdgme.Parser {
	tokens := append([]int{}, Gems...)
	if includeGold {
		tokens = append(tokens, Gold)
	}
	values := make([]brdgme.EnumValue, len(tokens))
	for i, t := range tokens {
		values[i] = brdgme.EnumValue{
			Name:  ResourceStrings[t],
			Value: t,
		}
	}
	return brdgme.Enum{
		Values: values,
	}
}

func TokensParser(includeGold bool) brdgme.Parser {
	var tokensParserMin = uint(1)
	return brdgme.Map{
		Parser: brdgme.Many{
			Parser: TokenParser(includeGold),
			Delim:  brdgme.Space{},
			Min:    &tokensParserMin,
		},
		Func: func(value interface{}) interface{} {
			tokens := []int{}
			for _, t := range value.([]interface{}) {
				tokens = append(tokens, t.(int))
			}
			return tokens
		},
	}
}

func (g *Game) BuyParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "buy",
				Desc:   "buy a card",
				Parser: brdgme.Token("buy"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "card",
					Desc:   "the card to buy",
					Parser: g.LocParser(player),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			loc := value.([]interface{})[1].(ParsedLoc)
			return BuyCommand(loc)
		},
	}
}

func (g *Game) DiscardParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "discard",
				Desc:   "discard tokens back down to 10",
				Parser: brdgme.Token("discard"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "tokens",
					Desc:   "the tokens to discard",
					Parser: TokensParser(true),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return DiscardCommand{
				Tokens: value.([]interface{})[1].([]int),
			}
		},
	}
}

func (g *Game) ReserveParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "reserve",
				Desc:   "reserve a card and take a gold",
				Parser: brdgme.Token("reserve"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "card",
					Desc:   "the card to reserve",
					Parser: g.LocParser(player),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			loc := value.([]interface{})[1].(ParsedLoc)
			return ReserveCommand(loc)
		},
	}
}

func (g *Game) TakeParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "take",
				Desc:   "take 3 different tokens, or 2 of the same token",
				Parser: brdgme.Token("take"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "tokens",
					Desc:   "the tokens to take",
					Parser: TokensParser(false),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return TakeCommand{
				Tokens: value.([]interface{})[1].([]int),
			}
		},
	}
}

func (g *Game) VisitParser(player int) brdgme.Parser {
	min := 1
	max := len(g.Nobles)
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "visit",
				Desc:   "visit a noble",
				Parser: brdgme.Token("visit"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "noble",
					Desc: "the noble to visit",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return VisitCommand{
				Noble: value.([]interface{})[1].(int) - 1,
			}
		},
	}
}
