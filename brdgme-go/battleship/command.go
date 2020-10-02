package battleship

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PlaceCommand struct {
	Ship, Y, X, Dir int
}

type ShootCommand struct {
	Y, X int
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
	case PlaceCommand:
		return g.PlaceCommand(player, value.Ship, value.Y, value.X, value.Dir, output.Remaining)
	case ShootCommand:
		return g.ShootCommand(player, value.Y, value.X, output.Remaining)
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

func (g *Game) PlaceCommand(player, ship, y, x, dir int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.PlaceShip(player, ship, y, x, dir)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) ShootCommand(player, y, x int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Shoot(player, y, x)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanPlace(player) {
		parsers = append(parsers, g.PlaceParser(player))
	}
	if g.CanShoot(player) {
		parsers = append(parsers, ShootParser())
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) PlayerRemainingShipParser(player int) brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, s := range g.LeftToPlace[player] {
		values = append(values, brdgme.EnumValue{
			Name:  shipNames[s],
			Value: s,
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}

type ParsedLoc struct {
	Y, X int
}

func LocParser() brdgme.Parser {
	values := []brdgme.EnumValue{}
	for y := 0; y < 10; y++ {
		for x := 0; x < 10; x++ {
			values = append(values, brdgme.EnumValue{
				Name: fmt.Sprintf("%c%d", 'A'+y, x+1),
				Value: ParsedLoc{
					Y: y,
					X: x,
				},
			})
		}
	}
	return brdgme.Enum{
		Values: values,
	}
}

func DirParser() brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, d := range directions {
		values = append(values, brdgme.EnumValue{
			Name:  directionNames[d],
			Value: d,
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}

func (g *Game) PlaceParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "place",
				Desc:   "place a ship",
				Parser: brdgme.Token("place"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "ship",
					Desc:   "the ship to place",
					Parser: g.PlayerRemainingShipParser(player),
				},
			),
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "location",
					Desc:   "the location to place the ship",
					Parser: LocParser(),
				},
			),
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "direction",
					Desc:   "the direction to place the ship",
					Parser: DirParser(),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			values := value.([]interface{})
			loc := values[2].(ParsedLoc)
			return PlaceCommand{
				Ship: values[1].(int),
				Y:    loc.Y,
				X:    loc.X,
				Dir:  values[3].(int),
			}
		},
	}
}

func ShootParser() brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "shoot",
				Desc:   "shoot at a location",
				Parser: brdgme.Token("shoot"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "location",
					Desc:   "the location to shoot at",
					Parser: LocParser(),
				},
			),
		},
		Func: func(value interface{}) interface{} {
			loc := value.([]interface{})[1].(ParsedLoc)
			return ShootCommand(loc)
		},
	}
}
