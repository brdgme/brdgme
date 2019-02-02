package age_of_war

import (
	"brdgme-go/brdgme"
)

type attackCommand struct {
	castle int
}

type lineCommand struct {
	line int
}

type rollCommand struct{}

func (g *Game) CommandParser(player int) brdgme.Parser {
	oneOf := brdgme.OneOf{}
	if g.CanAttack(player) {
		oneOf = append(oneOf, g.AttackParser())
	}
	if g.CanLine(player) {
		oneOf = append(oneOf, g.LineParser())
	}
	if g.CanRoll(player) {
		oneOf = append(oneOf, rollParser)
	}
	return oneOf
}

func (g *Game) CommandSpec(player int) *brdgme.Spec {
	spec := g.CommandParser(player).ToSpec()
	return &spec
}

func (g *Game) AttackParser() brdgme.Map {
	remainingCastles := []brdgme.EnumValue{}
	for k, c := range Castles {
		if g.Conquered[k] && g.CastleOwners[k] == g.CurrentPlayer {
			continue
		}
		if conquered, _ := g.ClanConquered(c.Clan); conquered {
			continue
		}
		remainingCastles = append(remainingCastles, brdgme.EnumValue{
			Name:  c.Name,
			Value: k,
		})
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "attack",
				Desc:   "attack a castle",
				Parser: brdgme.Token("attack"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name: "castle",
					Desc: "the castle to attack",
					Parser: brdgme.Enum{
						Values: remainingCastles,
					},
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return attackCommand{
				castle: value.([]interface{})[1].(int),
			}
		},
	}
}

func (g *Game) LineParser() brdgme.Map {
	remainingLines := []int{}
	castleLines := len(Castles[g.CurrentlyAttacking].CalcLines(
		g.Conquered[g.CurrentlyAttacking],
	))
	for i := 0; i < castleLines; i++ {
		if !g.CompletedLines[i] {
			remainingLines = append(remainingLines, i+1)
		}
	}
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "line",
				Desc:   "complete a castle line",
				Parser: brdgme.Token("line"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "line",
					Desc:   "the castle line to complete",
					Parser: brdgme.EnumFromInts(remainingLines, true),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			return lineCommand{
				line: value.([]interface{})[1].(int),
			}
		},
	}
}

var rollParser = brdgme.Map{
	Parser: brdgme.Doc{
		Name:   "roll",
		Desc:   "discard one dice and roll the rest",
		Parser: brdgme.Token("roll"),
	},
	Func: func(value interface{}) interface{} {
		return rollCommand{}
	},
}
