package roll_through_the_ages

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type BuildType = int

const (
	BuildTypeCity BuildType = iota
	BuildTypeShip
	BuildTypeMonument
)

type BuildCommand struct {
	Amount int
	Target BuildCommandTarget
}

type BuildCommandTarget struct {
	Type     BuildType
	Monument MonumentID
}

type BuyCommand struct {
	Development DevelopmentID
	Goods       BuyCommandGoods
}

type BuyCommandGoods struct {
	AllGoods bool
	Goods    []Good
}

type TradeCommand struct {
	Amount int
}

type NextCommand struct{}

type TakeCommand struct {
	Actions []TakeAction
}

type DiscardCommand struct {
	Amount int
	Good   Good
}

type InvadeCommand struct {
	Amount int
}

type RollCommand struct {
	Dice []int
}

type SellCommand struct {
	Amount int
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := []brdgme.Parser{}
	if g.CanBuild(player) {
		parsers = append(parsers, g.BuildParser(player))
	}
	if g.CanBuy(player) {
		parsers = append(parsers, g.BuyParser(player))
	}
	if g.CanTrade(player) {
		parsers = append(parsers, g.TradeParser(player))
	}
	if g.CanNext(player) {
		parsers = append(parsers, g.NextParser())
	}
	if g.CanTake(player) {
		parsers = append(parsers, g.TakeParser(player))
	}
	if g.CanDiscard(player) {
		parsers = append(parsers, g.DiscardParser(player))
	}
	if g.CanInvade(player) {
		parsers = append(parsers, g.InvadeParser(player))
	}
	if g.CanRoll(player) {
		parsers = append(parsers, g.RollParser(player))
	}
	if g.CanSell(player) {
		parsers = append(parsers, g.SellParser(player))
	}
	return brdgme.OneOf(parsers)
}

func (g *Game) BuildParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "build",
				Desc:   "build a city, monument or ship",
				Parser: brdgme.Token("build"),
			},
			brdgme.AfterSpace(
				brdgme.OneOf([]brdgme.Parser{
					g.BuildTargetWorkerParser(player),
					g.BuildTargetShipParser(player),
				}),
			),
		},
		Func: func(value interface{}) interface{} {
			return value.([]interface{})[1].(BuildCommand)
		},
	}
}

func (g *Game) BuildTargetWorkerParser(player int) brdgme.Parser {
	min := 1
	max := g.RemainingWorkers

	parsers := []brdgme.Parser{}
	if g.Boards[player].CityProgress < MaxCityProgress {
		parsers = append(parsers, BuildTargetCityParser)
	}
	if len(g.AvailableMonuments(player)) > 0 {
		parsers = append(parsers, g.BuildTargetMonumentParser(player))
	}

	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name: "amount",
				Desc: "the amount to build",
				Parser: brdgme.Int{
					Min: &min,
					Max: &max,
				},
			},
			brdgme.AfterSpace(brdgme.OneOf(parsers)),
		},
		Func: func(value interface{}) interface{} {
			return BuildCommand{
				Amount: value.([]interface{})[0].(int),
				Target: value.([]interface{})[1].(BuildCommandTarget),
			}
		},
	}
}

var BuildTargetCityParser = brdgme.Map{
	Parser: brdgme.Token("city"),
	Func: func(value interface{}) interface{} {
		return BuildCommandTarget{
			Type: BuildTypeCity,
		}
	},
}

func (g *Game) BuildTargetShipParser(player int) brdgme.Parser {
	min := 1
	max := g.Boards[player].Goods[GoodWood]
	if max < g.Boards[player].Goods[GoodCloth] {
		max = g.Boards[player].Goods[GoodCloth]
	}

	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name: "amount",
				Desc: "the amount of ships to build",
				Parser: brdgme.Int{
					Min: &min,
					Max: &max,
				},
			},
			brdgme.AfterSpace(brdgme.Token("ship")),
		},
		Func: func(value interface{}) interface{} {
			return BuildCommand{
				Amount: value.([]interface{})[0].(int),
				Target: BuildCommandTarget{
					Type: BuildTypeShip,
				},
			}
		},
	}
}

func (g *Game) BuildTargetMonumentParser(player int) brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, m := range g.AvailableMonuments(player) {
		values = append(values, brdgme.EnumValue{
			Name:  MonumentValues[m].Name,
			Value: m,
		})
	}
	return brdgme.Map{
		Parser: brdgme.Doc{
			Name: "monument",
			Desc: "the monument to build",
			Parser: brdgme.Enum{
				Values: values,
			},
		},
		Func: func(value interface{}) interface{} {
			return BuildCommandTarget{
				Type:     BuildTypeMonument,
				Monument: value.(MonumentID),
			}
		},
	}
}

func (g *Game) TradeParser(player int) brdgme.Parser {
	min := 1
	max := g.Boards[player].Goods[GoodStone]
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "trade",
				Desc:   "trade stone for 3 workers each",
				Parser: brdgme.Token("trade"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "the amount of stone to trade",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return TradeCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}

func (g *Game) BuyParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "buy",
				Desc:   "buy a development",
				Parser: brdgme.Token("buy"),
			},
			brdgme.AfterSpace(
				g.BuyDevelopmentParser(player),
			),
			brdgme.Opt{
				Parser: brdgme.AfterSpace(g.BuyGoodParser(player)),
			},
		},
		Func: func(value interface{}) interface{} {
			goods := BuyCommandGoods{}
			parsedGoods := value.([]interface{})[2]
			if parsedGoods != nil {
				goods = parsedGoods.(BuyCommandGoods)
			}
			return BuyCommand{
				Development: value.([]interface{})[1].(DevelopmentID),
				Goods:       goods,
			}
		},
	}
}

func (g *Game) BuyDevelopmentParser(player int) brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, d := range g.AvailableDevelopments(player) {
		values = append(values, brdgme.EnumValue{
			Name:  DevelopmentValues[d].Name,
			Value: d,
		})
	}
	return brdgme.Map{
		Parser: brdgme.Doc{
			Name: "development",
			Desc: "the development to buy",
			Parser: brdgme.Enum{
				Values: values,
			},
		},
		Func: func(value interface{}) interface{} {
			return value.(DevelopmentID)
		},
	}
}

func (g *Game) BuyGoodParser(player int) brdgme.Parser {
	var min uint = 1
	return brdgme.OneOf{
		brdgme.Map{
			Parser: brdgme.Token("all"),
			Func: func(value interface{}) interface{} {
				return BuyCommandGoods{
					AllGoods: true,
				}
			},
		},
		brdgme.Map{
			Parser: brdgme.Many{
				Min:    &min,
				Parser: GoodParser(),
			},
			Func: func(value interface{}) interface{} {
				goods := []Good{}
				for _, g := range value.([]interface{}) {
					goods = append(goods, g.(Good))
				}
				return BuyCommandGoods{
					Goods: goods,
				}
			},
		},
	}
}

func GoodParser() brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, good := range Goods {
		values = append(values, brdgme.EnumValue{
			Name:  GoodStrings[good],
			Value: good,
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}

func (g *Game) NextParser() brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Doc{
			Name:   "next",
			Desc:   "continue to the next phase of your turn",
			Parser: brdgme.Token("next"),
		},
		Func: func(value interface{}) interface{} {
			return NextCommand{}
		},
	}
}

func (g *Game) TakeParser(player int) brdgme.Parser {
	var min uint = 1
	var max uint = 0
	for _, v := range g.KeptDice {
		if v == DiceFoodOrWorkers {
			max += 1
		}
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "take",
				Desc:   "take food or workers from dice",
				Parser: brdgme.Token("take"),
			},
			brdgme.AfterSpace(
				brdgme.Many{
					Min: &min,
					Max: &max,
					Parser: brdgme.Enum{
						Values: []brdgme.EnumValue{
							{Name: TakeMap[TakeFood], Value: TakeFood},
							{Name: TakeMap[TakeWorkers], Value: TakeWorkers},
						},
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			actions := []TakeAction{}
			for _, v := range value.([]interface{})[1].([]interface{}) {
				actions = append(actions, v.(TakeAction))
			}
			return TakeCommand{Actions: actions}
		},
	}
}

func (g *Game) DiscardParser(player int) brdgme.Parser {
	min := 1
	max := g.Boards[player].GoodsOverLimit()
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "discard",
				Desc:   "discard goods down to the limit",
				Parser: brdgme.Token("discard"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "amount of goods to discard",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "good",
					Desc:   "type of good to discard",
					Parser: GoodParser(),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return DiscardCommand{
				Amount: value.([]interface{})[1].(int),
				Good:   value.([]interface{})[2].(Good),
			}
		},
	}
}

func (g *Game) InvadeParser(player int) brdgme.Parser {
	min := 1
	max := g.Boards[player].Goods[GoodSpearhead]
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "invade",
				Desc:   "invade other players by spending spearheads for -2 points each",
				Parser: brdgme.Token("invade"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "amount of spearheads to spend",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return InvadeCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}

func (g *Game) RollParser(player int) brdgme.Parser {
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
				brdgme.Many{
					Min: &minU,
					Max: &maxU,
					Parser: brdgme.Int{
						Min: &minI,
						Max: &maxI,
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

func (g *Game) SellParser(player int) brdgme.Parser {
	min := 1
	max := g.Boards[player].Food
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "sell",
				Desc:   "sell food for 6 coins each",
				Parser: brdgme.Token("sell"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "amount",
					Desc: "amount of food to sell",
					Parser: brdgme.Int{
						Min: &min,
						Max: &max,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return SellCommand{
				Amount: value.([]interface{})[1].(int),
			}
		},
	}
}
