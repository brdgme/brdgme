package cathedral_1

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type Dir int

const (
	DirUp Dir = 1 << iota
	DirRight
	DirDown
	DirLeft
)

var OrthoDirs = []Dir{
	DirUp,
	DirRight,
	DirDown,
	DirLeft,
}

var OrthoDirNames = map[Dir]string{
	DirUp:    "up",
	DirRight: "right",
	DirDown:  "down",
	DirLeft:  "left",
}

var DiagDirs = []Dir{
	DirUp | DirRight,
	DirDown | DirRight,
	DirDown | DirLeft,
	DirUp | DirLeft,
}

var Dirs = append(append([]Dir{}, OrthoDirs...), DiagDirs...)

type Tiler interface {
	TileAt(loc Loc) (Tile, bool)
}

func DirInv(dir Dir) Dir {
	var inv Dir
	if dir&DirUp > 0 {
		inv = inv | DirDown
	}
	if dir&DirRight > 0 {
		inv = inv | DirLeft
	}
	if dir&DirDown > 0 {
		inv = inv | DirUp
	}
	if dir&DirLeft > 0 {
		inv = inv | DirRight
	}
	return inv
}

type Game struct {
	Players int

	Board Board

	PlayedPieces map[int]map[int]bool

	CurrentPlayer int

	NoOpenTiles bool
	Finished    bool
}

var _ brdgme.Gamer = &Game{}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players != 2 {
		return nil, errors.New("Cathedral is two player")
	}
	g.Players = players

	g.Board = Board{}
	for _, l := range AllLocs {
		g.Board[l.String()] = EmptyTile
	}
	g.PlayedPieces = map[int]map[int]bool{}
	for p := 0; p < g.Players; p++ {
		g.PlayedPieces[p] = map[int]bool{}
	}

	return nil, nil
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2}
}

func (g *Game) PlayerState(player int) interface{} {
	return nil
}

func (g *Game) PubState() interface{} {
	return nil
}

func (g *Game) Points() []float32 {
	points := make([]float32, g.Players)
	for p := 0; p < g.Players; p++ {
		points[p] = float32(g.RemainingPieceSize(p))
	}
	return points
}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		return brdgme.StatusFinished{
			Placings: g.Placings(),
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn: g.WhoseTurn(),
	}.ToStatus()
}

func (g *Game) IsFinished() bool {
	return g.Finished
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{-g.RemainingPieceSize(p)}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	if g.NoOpenTiles {
		players := []int{}
		for p := 0; p < g.Players; p++ {
			if g.CanPlaySomething(p, LocFilterPlayable) {
				players = append(players, p)
			}
		}
		return players
	}
	return []int{g.CurrentPlayer}
}

func Neighbour(src Tiler, loc Loc, dir Dir) (Tile, bool) {
	return src.TileAt(loc.Neighbour(dir))
}

func OpenSides(src Tiler, loc Loc) (open map[Dir]bool) {
	t, ok := src.TileAt(loc)
	if !ok {
		return
	}
	open = map[Dir]bool{}
	for _, d := range Dirs {
		if nt, ok := Neighbour(src, loc, d); ok && t.Player == nt.Player &&
			t.Type == nt.Type {
			open[d] = true
		}
	}
	return
}

func (g *Game) NextPlayer() {
	opponent := Opponent(g.CurrentPlayer)
	// We switch to the opponent if they have any playable pieces.
	if g.CanPlaySomething(opponent, LocFilterPlayable) {
		g.CurrentPlayer = opponent
	}
}

func (g *Game) RemainingPieceSize(player int) int {
	sum := 0
	for pNum, piece := range Pieces[player] {
		if !g.PlayedPieces[player][pNum] {
			sum += len(piece.Positions)
		}
	}
	return sum
}

type LocFilter func(g *Game, player int, loc Loc) bool

func LocFilterPlayable(g *Game, player int, loc Loc) bool {
	t := g.Board[loc.String()]
	return t.Player == NoPlayer && (t.Owner == NoPlayer || t.Owner == player)
}

func LocFilterOpen(g *Game, player int, loc Loc) bool {
	t := g.Board[loc.String()]
	return t.Player == NoPlayer && t.Owner == NoPlayer
}

func (g *Game) CanPlaySomething(player int, filter LocFilter) bool {
	for _, l := range AllLocs {
		if !filter(g, player, l) {
			continue
		}
		// Try to play the easiest one first
		for i := len(Pieces[player]) - 1; i >= 0; i-- {
			if g.PlayedPieces[player][i] {
				continue
			}
			dirs := OrthoDirs
			if !Pieces[player][i].Directional {
				dirs = []Dir{DirDown}
			}
			for _, dir := range dirs {
				if ok, _ := g.CanPlayPiece(player, i, l, dir); ok {
					return true
				}
			}
		}
	}
	return false
}

func Opponent(pNum int) int {
	return (pNum + 1) % 2
}
