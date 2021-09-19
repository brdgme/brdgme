package category_5_1

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func (g *Game) PlayerRender(pNum int) string {
	buf := bytes.NewBuffer([]byte{})
	// Board
	cells := [][]render.Cell{}
	for i, b := range g.Board {
		row := []render.Cell{
			render.Cel(render.Bold(fmt.Sprintf("#%d", i+1))),
		}
		for j := 0; j < 5; j++ {
			cellContent := "  "
			if j < len(b) {
				cellContent = b[j].String()
			}
			row = append(row, render.Cel(cellContent))
		}
		row = append(row, render.Cel(fmt.Sprintf("  %d pts", CardsHeads(b))))
		cells = append(cells, row)
	}
	buf.WriteString(render.Table(cells, 0, 2))
	if pNum >= 0 {
		// Hand
		if len(g.Hands[pNum]) > 0 {
			buf.WriteString("\n\n")
			row := []render.Cell{
				render.Cel(render.Bold("Your hand:")),
			}
			for _, c := range g.Hands[pNum] {
				row = append(row, render.Cel(c.String()))
			}
			buf.WriteString(render.Table([][]render.Cell{row}, 0, 2))
		}
	}
	// Legend
	buf.WriteString("\n\n")
	parts := []string{}
	for _, i := range []int{1, 2, 3, 5, 7} {
		parts = append(parts, render.Markup(
			fmt.Sprintf("%d pts", i),
			CardColours[i],
			true,
		))
	}
	buf.WriteString(fmt.Sprintf(
		"%s %s",
		render.Bold("Legend:"),
		strings.Join(parts, ", "),
	))
	// Score table
	buf.WriteString("\n\n")
	cells = [][]render.Cell{{
		render.Cel(render.Bold("Players")),
		render.Cel(render.Bold("Taken")),
		render.Cel(render.Bold("Pts")),
	}}
	for p := 0; p < g.Players; p++ {
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(p)),
			render.Cel(strconv.Itoa(len(g.PlayerCards[p])), render.Center),
			render.Cel(render.Bold(strconv.Itoa(g.PlayerPoints[p])), render.Center),
		})
	}
	buf.WriteString(render.Table(cells, 0, 2))
	points := []int{}
	for p := 0; p < g.Players; p++ {
		points = append(points, g.PlayerPoints[p])
	}
	buf.WriteString(fmt.Sprintf(
		"\n\n%s until the end of the game.",
		render.Bold(fmt.Sprintf("%d points", EndScore-brdgme.IntMax(points...))),
	))
	return buf.String()
}
