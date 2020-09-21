package modern_art

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
)

type AddCommand struct {
	Card libcard.Card
}

type BidCommand struct {
	Amount int
}

type BuyCommand struct{}

type PassCommand struct{}

type PlayCommand struct {
	Card libcard.Card
}

type PriceCommand struct {
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
	case AddCommand:
		return g.AddCommand(player, value.Card, output.Remaining)
	case BidCommand:
		return g.BidCommand(player, value.Amount, output.Remaining)
	case BuyCommand:
		return g.BuyCommand(player, output.Remaining)
	case PassCommand:
		return g.PassCommand(player, output.Remaining)
	case PlayCommand:
		return g.PlayCommand(player, value.Card, output.Remaining)
	case PriceCommand:
		return g.PriceCommand(player, value.Amount, output.Remaining)
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

func (g *Game) AddCommand(player int, card libcard.Card, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.AddCard(player, card)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) BidCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Bid(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) BuyCommand(player int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Buy(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
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

func (g *Game) PlayCommand(player int, card libcard.Card, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlayCard(player, card)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) PriceCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.SetPrice(player, amount)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanAdd(player) {
		parsers = append(parsers, g.AddParser(player))
	}
	if g.CanBid(player) {
		parsers = append(parsers, g.BidParser(player))
	}
	if g.CanBuy(player) {
		parsers = append(parsers, BuyParser)
	}
	if g.CanPass(player) {
		parsers = append(parsers, PassParser)
	}
	if g.CanPlay(player) {
		parsers = append(parsers, g.PlayParser(player))
	}
	if g.CanSetPrice(player) {
		parsers = append(parsers, g.PriceParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func CardParser(card libcard.Card) brdgme.Parser {
	return brdgme.Doc{
		Desc:   fmt.Sprintf("%s - %s", suitNames[card.Suit], rankNames[card.Rank]),
		Parser: brdgme.Token(fmt.Sprintf("%s%s", suitCodes[card.Suit], rankCodes[card.Rank])),
	}
}

func CardsParser(cards libcard.Deck) brdgme.Parser {
	parsers := make([]brdgme.Parser, len(cards))
	for i, c := range cards.Sort() {
		parsers[i] = CardParser(c)
	}
	return brdgme.OneOf(parsers)
}

func (g *Game) AddParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "add",
				Desc:   "add a card from the same artist to the auction",
				Parser: brdgme.Token("add"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "card",
					Desc:   "the card to add",
					Parser: CardsParser(g.PlayerHands[player]),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return AddCommand{
				Card: value.([]interface{})[1].(libcard.Card),
			}
		},
	}
}

func (g *Game) BidParser(player int) brdgme.Parser {
	_, min := g.HighestBidder()
	min += 1
	if g.AuctionType() == RANK_SEALED {
		min = 1
	}
	max := g.PlayerMoney[player]
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "bid",
				Desc:   "bid for an artwork",
				Parser: brdgme.Token("bid"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "the amount to bid",
					Parser: brdgme.Int{
						Min: &min,
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

var BuyParser = brdgme.Doc{
	Name: "buy",
	Desc: "buy the painting for the asking price",
	Parser: brdgme.Map{
		Parser: brdgme.Token("buy"),
		Func: func(value interface{}) interface{} {
			return BuyCommand{}
		},
	},
}

var PassParser = brdgme.Doc{
	Name: "pass",
	Desc: "pass and leave the auction",
	Parser: brdgme.Map{
		Parser: brdgme.Token("pass"),
		Func: func(value interface{}) interface{} {
			return PassCommand{}
		},
	},
}

func (g *Game) PlayParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "play",
				Desc:   "play a card from your hand and put it up for auction",
				Parser: brdgme.Token("play"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "card",
					Desc:   "the card to play",
					Parser: CardsParser(g.PlayerHands[player]),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return PlayCommand{
				Card: value.([]interface{})[1].(libcard.Card),
			}
		},
	}
}

func (g *Game) PriceParser(player int) brdgme.Parser {
	min := 1
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "price",
				Desc:   "set the asking price for the artwork",
				Parser: brdgme.Token("price"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "the amount to set as the asking price",
					Parser: brdgme.Int{
						Min: &min,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return PriceCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}
