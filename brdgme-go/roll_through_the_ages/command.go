package roll_through_the_ages

import "github.com/brdgme/brdgme/brdgme-go/brdgme"

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
	AllGoods    bool
	Goods       []Good
}

type TradeCommand struct {
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
				Amount: value.([]interface{})[1].(int),
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
				Target: value.([]interface{})[1].(BuildCommandTarget),
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
				brdgme.OneOf([]brdgme.Parser{
					g.BuildTargetWorkerParser(player),
					g.BuildTargetShipParser(player),
				}),
			),
		},
		Func: func(value interface{}) interface{} {
			return value.([]interface{})[1].(BuyCommand)
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
