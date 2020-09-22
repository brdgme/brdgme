package love_letter

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PrincessCommand struct{}

type CountessCommand struct{}

type KingCommand struct {
	Target int
}

type PrinceCommand struct {
	Target int
}

type HandmaidCommand struct{}

type BaronCommand struct {
	Target int
}

type PriestCommand struct {
	Target int
}

type GuardCommand struct {
	Target, Card int
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
	case PrincessCommand:
		return g.PrincessCommand(player, output.Remaining)
	case CountessCommand:
		return g.CountessCommand(player, output.Remaining)
	case KingCommand:
		return g.KingCommand(player, value.Target, output.Remaining)
	case PrinceCommand:
		return g.PrinceCommand(player, value.Target, output.Remaining)
	case HandmaidCommand:
		return g.HandmaidCommand(player, output.Remaining)
	case BaronCommand:
		return g.BaronCommand(player, value.Target, output.Remaining)
	case PriestCommand:
		return g.PriestCommand(player, value.Target, output.Remaining)
	case GuardCommand:
		return g.GuardCommand(player, value.Target, value.Card, output.Remaining)
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

func (g *Game) PrincessCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayPrincess(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CountessCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayCountess(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) KingCommand(player, target int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayKing(player, target)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) PrinceCommand(player, target int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayPrince(player, target)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) HandmaidCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayHandmaid(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) BaronCommand(player, target int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayBaron(player, target)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) PriestCommand(player, target int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayPriest(player, target)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) GuardCommand(player, target, card int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayGuard(player, target, card)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanPlay(player) {
		if _, ok := brdgme.IntFind(Princess, g.Hands[player]); ok {
			parsers = append(parsers, PrincessParser)
		}
		if _, ok := brdgme.IntFind(Countess, g.Hands[player]); ok {
			parsers = append(parsers, CountessParser)
		}
		if _, ok := brdgme.IntFind(King, g.Hands[player]); ok {
			parsers = append(parsers, KingParser)
		}
		if _, ok := brdgme.IntFind(Prince, g.Hands[player]); ok {
			parsers = append(parsers, PrinceParser)
		}
		if _, ok := brdgme.IntFind(Handmaid, g.Hands[player]); ok {
			parsers = append(parsers, HandmaidParser)
		}
		if _, ok := brdgme.IntFind(Baron, g.Hands[player]); ok {
			parsers = append(parsers, BaronParser)
		}
		if _, ok := brdgme.IntFind(Priest, g.Hands[player]); ok {
			parsers = append(parsers, PriestParser)
		}
		if _, ok := brdgme.IntFind(Guard, g.Hands[player]); ok {
			parsers = append(parsers, GuardParser)
		}
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

var PrincessParser = brdgme.Map{
	Parser: brdgme.Doc{
		Name:   "princess",
		Desc:   "play the Princess card (you will be eliminated)",
		Parser: brdgme.Token("princess"),
	},
	Func: func(value interface{}) interface{} {
		return PrincessCommand{}
	},
}

var CountessParser = brdgme.Map{
	Parser: brdgme.Doc{
		Name:   "countess",
		Desc:   "play the Countess card, which you must do if you also have the King or Prince",
		Parser: brdgme.Token("countess"),
	},
	Func: func(value interface{}) interface{} {
		return CountessCommand{}
	},
}

var KingParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "king",
			Desc:   "play the King card to trade your hand with another player",
			Parser: brdgme.Token("king"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "player",
				Desc:   "the player to trade hands with",
				Parser: brdgme.Player{},
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return KingCommand{
			Target: value.([]interface{})[1].(int),
		}
	},
}

var PrinceParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "prince",
			Desc:   "play the Prince card to make a player discard their hand, including yourself",
			Parser: brdgme.Token("prince"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "player",
				Desc:   "the player to discard their hand, including yourself",
				Parser: brdgme.Player{},
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return PrinceCommand{
			Target: value.([]interface{})[1].(int),
		}
	},
}

var HandmaidParser = brdgme.Map{
	Parser: brdgme.Doc{
		Name:   "handmaid",
		Desc:   "play the Handmaid card, which protects you from being targeted until your next turn",
		Parser: brdgme.Token("handmaid"),
	},
	Func: func(value interface{}) interface{} {
		return HandmaidCommand{}
	},
}

var BaronParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "baron",
			Desc:   "play the Baron card to compare your hand to another player, lowest hand is eliminated",
			Parser: brdgme.Token("baron"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "player",
				Desc:   "the player to compare your hand to",
				Parser: brdgme.Player{},
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return BaronCommand{
			Target: value.([]interface{})[1].(int),
		}
	},
}

var PriestParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "priest",
			Desc:   "play the Baron card to peek at another player's hand",
			Parser: brdgme.Token("priest"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "player",
				Desc:   "the player to peek",
				Parser: brdgme.Player{},
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return PriestCommand{
			Target: value.([]interface{})[1].(int),
		}
	},
}

func CardParserValues() []brdgme.EnumValue {
	values := []brdgme.EnumValue{}
	for c := Princess; c >= Guard; c-- {
		values = append(values, brdgme.EnumValue{
			Name:  Cards[c].Name,
			Value: c,
		})
	}
	return values
}

var CardParser = brdgme.Enum{
	Values: CardParserValues(),
}

var GuardParser = brdgme.Map{
	Parser: brdgme.Chain{
		brdgme.Doc{
			Name:   "guard",
			Desc:   "play the Guard card to guess the card of another player, eliminating them if correct",
			Parser: brdgme.Token("guard"),
		},
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "player",
				Desc:   "the player to target",
				Parser: brdgme.Player{},
			},
		),
		brdgme.AfterSpace(
			brdgme.Doc{
				Name:   "card",
				Desc:   "the card you think they are",
				Parser: CardParser,
			},
		),
	},
	Func: func(value interface{}) interface{} {
		return GuardCommand{
			Target: value.([]interface{})[1].(int),
			Card:   value.([]interface{})[2].(int),
		}
	},
}
