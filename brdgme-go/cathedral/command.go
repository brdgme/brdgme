package cathedral

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type PlayCommand struct {
	Piece int
	Loc   Loc
	Dir   Dir
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
		return g.PlayCommand(player, value.Piece, value.Loc, value.Dir, output.Remaining)
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

func (g *Game) PlayCommand(player, piece int, loc Loc, dir Dir, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.Play(player, piece, loc, dir)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, err
}

func (g *Game) CommandParser(player int) brdgme.Parser {
	parsers := brdgme.OneOf{}
	if g.CanPlay(player) {
		parsers = append(parsers, g.PlayParser(player))
	}
	if len(parsers) == 0 {
		return nil
	}
	return parsers
}

func (g *Game) PlayParser(player int) brdgme.Parser {
	return brdgme.Map{
		Parser: brdgme.Chain{
			brdgme.Doc{
				Name:   "play",
				Desc:   "play a piece to the board",
				Parser: brdgme.Token("play"),
			},
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "piece",
					Desc:   "the piece to play",
					Parser: g.PieceParser(player),
				},
			),
			brdgme.AfterSpace(
				brdgme.Doc{
					Name:   "loc",
					Desc:   "the location to play at",
					Parser: g.LocParser(player),
				},
			),
			brdgme.Opt{
				Parser: brdgme.AfterSpace(
					brdgme.Doc{
						Name:   "dir",
						Desc:   "the direction to play the piece, or down if not specified",
						Parser: DirParser(),
					},
				),
			},
		},
		Func: func(value interface{}) interface{} {
			dir := DirDown
			if d, ok := value.([]interface{})[3].(Dir); ok {
				dir = d
			}
			return PlayCommand{
				Piece: value.([]interface{})[1].(int),
				Loc:   value.([]interface{})[2].(Loc),
				Dir:   dir,
			}
		},
	}
}

func (g *Game) PieceParser(player int) brdgme.Parser {
	min := 1
	max := len(Pieces[player])
	return brdgme.Map{
		Parser: brdgme.Int{
			Min: &min,
			Max: &max,
		},
		Func: func(value interface{}) interface{} {
			return value.(int) - 1
		},
	}
}

func (g *Game) LocParser(player int) brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, l := range AllLocs {
		values = append(values, brdgme.EnumValue{
			Value: l,
			Name:  l.String(),
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}

func DirParser() brdgme.Parser {
	values := []brdgme.EnumValue{}
	for _, d := range OrthoDirs {
		values = append(values, brdgme.EnumValue{
			Value: d,
			Name:  OrthoDirNames[d],
		})
	}
	return brdgme.Enum{
		Values: values,
	}
}
