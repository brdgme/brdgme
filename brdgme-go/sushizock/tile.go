package sushizock

import (
	"math/rand"
	"strconv"
	"strings"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

type TileType int

const (
	TileTypeBlue TileType = iota
	TileTypeRed
)

var TileColors = map[TileType]render.Color{
	TileTypeBlue: render.Blue,
	TileTypeRed:  render.Red,
}

type Tile struct {
	Type  TileType
	Value int
}

func (t Tile) Render() string {
	return render.Fg(TileColors[t.Type], strconv.Itoa(t.Value))
}

type Tiles []Tile

func (tiles Tiles) Cells() []render.Cell {
	cells := make([]render.Cell, len(tiles))
	for i, t := range tiles {
		cells[i] = render.Cel(t.Render())
	}
	return cells
}

func (tiles Tiles) RenderSlice() []string {
	strs := make([]string, len(tiles))
	for i, t := range tiles {
		strs[i] = t.Render()
	}
	return strs
}

func (tiles Tiles) Render() string {
	return strings.Join(tiles.RenderSlice(), " ")
}

func (tiles Tiles) Sum() int {
	sum := 0
	for _, t := range tiles {
		sum += t.Value
	}
	return sum
}

func (tiles Tiles) Remove(i int) (Tile, Tiles) {
	t := tiles[i]
	remaining := append(append(Tiles{}, tiles[:i]...), tiles[i+1:]...)
	return t, remaining
}

func BlueTiles() Tiles {
	return Tiles{
		{TileTypeBlue, 1},
		{TileTypeBlue, 1},
		{TileTypeBlue, 2},
		{TileTypeBlue, 2},
		{TileTypeBlue, 3},
		{TileTypeBlue, 3},
		{TileTypeBlue, 4},
		{TileTypeBlue, 4},
		{TileTypeBlue, 5},
		{TileTypeBlue, 5},
		{TileTypeBlue, 6},
		{TileTypeBlue, 6},
	}
}

func RedTiles() Tiles {
	return Tiles{
		{TileTypeRed, -1},
		{TileTypeRed, -1},
		{TileTypeRed, -1},
		{TileTypeRed, -1},
		{TileTypeRed, -1},
		{TileTypeRed, -2},
		{TileTypeRed, -2},
		{TileTypeRed, -2},
		{TileTypeRed, -2},
		{TileTypeRed, -3},
		{TileTypeRed, -3},
		{TileTypeRed, -4},
	}
}

func ShuffleTiles(tiles Tiles) Tiles {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	shuffled := make(Tiles, len(tiles))
	for i, p := range r.Perm(len(tiles)) {
		shuffled[i] = tiles[p]
	}
	return shuffled
}

func Score(blue, red Tiles) int {
	score := 0
	for _, r := range red {
		score += r.Value
	}
	for i, b := range blue {
		if i < len(red) {
			score += b.Value
		}
	}
	return score
}
