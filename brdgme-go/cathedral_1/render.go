package cathedral_1

import (
	"bytes"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	TileWidth  = 6
	TileHeight = 3
)

var (
	NoTileStr       = ` `
	PieceBackground = ` `
)

var WallStrs = map[Dir]string{
	DirUp | DirDown | DirLeft | DirRight: render.Bold("+"),
	DirUp | DirDown | DirLeft:            render.Bold("+"),
	DirUp | DirDown | DirRight:           render.Bold("+"),
	DirUp | DirLeft | DirRight:           render.Bold("+"),
	DirDown | DirLeft | DirRight:         render.Bold("+"),
	DirUp | DirLeft:                      render.Bold("+"),
	DirUp | DirRight:                     render.Bold("+"),
	DirDown | DirLeft:                    render.Bold("+"),
	DirDown | DirRight:                   render.Bold("+"),
	DirLeft | DirRight:                   render.Bold("-"),
	DirLeft:                              render.Bold("-"),
	DirRight:                             render.Bold("-"),
	DirUp | DirDown:                      render.Bold("|"),
	DirUp:                                render.Bold("|"),
	DirDown:                              render.Bold("|"),
}

var SideWall = render.Bold(strings.TrimSpace(strings.Repeat(fmt.Sprintf(
	"%s\n",
	WallStrs[DirUp|DirDown],
), TileHeight)))

func (g *Game) PlayerRender(pNum int) string {
	buf := bytes.NewBuffer([]byte{})
	buf.WriteString(g.Board.Render())
	buf.WriteString("\n\nAll pieces are shown in their {{b}}down{{/b}} position and pivot around the number.")
	buf.WriteString(render.Bold(fmt.Sprintf(
		"\n\n%s remaining tiles:\n",
		render.Player(pNum),
	)))
	buf.WriteString(g.RenderPlayerRemainingTiles(pNum))
	buf.WriteString(render.Bold(fmt.Sprintf(
		"\n\n%s remaining tiles:\n",
		render.Player(Opponent(pNum)),
	)))
	buf.WriteString(g.RenderPlayerRemainingTiles(Opponent(pNum)))
	return buf.String()
}

func (g *Game) PubRender() string {
	return g.PlayerRender(0)
}

func (g *Game) RenderPlayerRemainingTiles(pNum int) string {
	buf := bytes.NewBuffer([]byte{})
	cells := [][]render.Cell{{}}
	curWidth := 0
	hasTiles := false
	for i, p := range Pieces[pNum] {
		if g.PlayedPieces[pNum][i] {
			continue
		}
		hasTiles = true
		pWidth := p.Width()
		if curWidth+pWidth > 10 {
			buf.WriteString("\n")
			buf.WriteString(render.Table(cells, 0, 2))
			cells = [][]render.Cell{{}}
			curWidth = 0
		}
		cells[0] = append(cells[0], render.Cel(p.Render()))
		curWidth += pWidth
	}
	if !hasTiles {
		return render.Markup("None", render.Grey, true)
	}
	if len(cells) > 0 {
		buf.WriteString("\n")
		buf.WriteString(render.Table(cells, 0, 2))
	}
	return buf.String()
}

var (
	emptyAbove = (TileHeight - 1) / 2
	emptyBelow = TileHeight / 2
)

func RenderTile(src Tiler, loc Loc) (string, bool) {
	t, ok := src.TileAt(loc)
	if !ok || t.Player == NoPlayer {
		return "", false
	}
	return RenderPlayerTile(t, OpenSides(src, loc)), true
}

func RenderPlayerTile(tile Tile, open map[Dir]bool) string {
	// Top row
	buf := bytes.NewBufferString(RenderCorner(DirUp|DirLeft, open))
	c := WallStrs[DirLeft|DirRight]
	if open[DirUp] {
		c = PieceBackground
	}
	buf.WriteString(strings.Repeat(c, TileWidth-2))
	buf.WriteString(RenderCorner(DirUp|DirRight, open))
	buf.WriteString("\n")

	// Middle rows
	left := WallStrs[DirUp|DirDown]
	if open[DirLeft] {
		left = PieceBackground
	}
	right := WallStrs[DirUp|DirDown]
	if open[DirRight] {
		right = PieceBackground
	}
	middleRow := fmt.Sprintf(
		"%s%s%s\n",
		left,
		render.Align(
			render.Center,
			TileWidth-2,
			tile.Text,
		),
		right,
	)
	buf.WriteString(strings.Repeat(middleRow, TileHeight-2))

	// Bottom row
	buf.WriteString(RenderCorner(DirDown|DirLeft, open))
	c = WallStrs[DirLeft|DirRight]
	if open[DirDown] {
		c = PieceBackground
	}
	buf.WriteString(strings.Repeat(c, TileWidth-2))
	buf.WriteString(RenderCorner(DirDown|DirRight, open))

	return render.Bold(render.Fgp(
		tile.Player,
		render.Bgp(tile.Player, buf.String()),
		render.Mono,
		render.Inv,
	))
}

func RenderCorner(dir Dir, open map[Dir]bool) string {
	// If all three tiles in dir are open, then render nothing.
	numOpen := 0
	for _, d := range Dirs {
		if dir&d == d && open[d] {
			numOpen++
			if numOpen == 3 {
				return PieceBackground
			}
		}
	}

	// Map of one corner direction referencing the other.
	cornerMap := map[Dir]Dir{}
	first := Dir(-1)
	for _, d := range Dirs {
		if dir&d != d {
			continue
		}
		if first == -1 {
			first = d
		} else {
			cornerMap[first] = d
			cornerMap[d] = first
			break
		}
	}

	var corner Dir
	for d, other := range cornerMap {
		if open[d] {
			corner = corner | d
		} else {
			corner = corner | DirInv(other)
		}
	}
	return WallStrs[corner]
}

func RenderEmptyTile(loc Loc, owner int) string {
	buf := bytes.NewBufferString(strings.Repeat(fmt.Sprintf(
		"%s\n",
		strings.Repeat(NoTileStr, TileWidth),
	), emptyAbove))
	s := loc.String()
	remainingWidth := TileWidth - len(s)
	buf.WriteString(strings.Repeat(NoTileStr, remainingWidth/2))
	if owner == NoPlayer {
		// Grey
		buf.WriteString(render.Markup(s, render.Grey, true))
	} else {
		// Player colour
		buf.WriteString(render.Bold(render.Fgp(owner, s)))
	}
	buf.WriteString(strings.Repeat(NoTileStr, (remainingWidth+1)/2))
	buf.WriteByte('\n')
	buf.WriteString(strings.TrimSpace(strings.Repeat(fmt.Sprintf(
		"%s\n",
		strings.Repeat(NoTileStr, TileWidth),
	), emptyBelow)))
	return render.Fg(render.Grey, buf.String())
}
