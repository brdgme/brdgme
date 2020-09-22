package zombie_dice

import (
	"bytes"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var DiceFaceStrings = map[int]string{
	Brain:      "Brain",
	Shotgun:    "Shot",
	Footprints: "Run",
}

func (g *Game) PlayerRender(player int) string {
	return g.PubRender()
}

func (g *Game) PubRender() string {
	output := bytes.NewBuffer([]byte{})
	cupStr := render.Markup("None", render.Grey, false)
	if len(g.Cup) > 0 {
		counts := make([]int, 3)
		for _, d := range g.Cup {
			counts[ColourOrder[d.Colour]]++
		}
		parts := []string{}
		for _, c := range Colours {
			if counts[ColourOrder[c]] > 0 {
				parts = append(parts, render.Markup(
					fmt.Sprintf("%d %s", counts[ColourOrder[c]], ColorNames[c]),
					c,
					true,
				))
			}
		}
		cupStr = brdgme.CommaList(parts)
	}
	output.WriteString(render.Table([][]render.Cell{
		{
			render.Cel("Brains", render.Right),
			render.Cel(render.Bold(strconv.Itoa(g.RoundBrains))),
		},
		{
			render.Cel("Shots:", render.Right),
			render.Cel(render.Bold(strconv.Itoa(g.RoundShotguns))),
		},
		{
			render.Cel("Runners:", render.Right),
			render.Cel(g.CurrentRoll.String()),
		},
		{
			render.Cel("Kept:", render.Right),
			render.Cel(g.Kept.String()),
		},
		{
			render.Cel("In cup:", render.Right),
			render.Cel(cupStr),
		},
	}, 0, 2))
	output.WriteString(render.Bold("\n\n\nScores:\n"))
	cells := [][]render.Cell{}
	for p := 0; p < g.Players; p++ {
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(p), render.Right),
			render.Cel(render.Bold(strconv.Itoa(g.Scores[p]))),
		})
	}
	output.WriteString(render.Table(cells, 0, 2))
	return output.String()
}

func (d DiceResult) String() string {
	return render.Markup(DiceFaceStrings[d.Face], d.Colour, true)
}

func (drl DiceResultList) String() string {
	parts := make([]string, len(drl))
	for i, dr := range drl {
		parts[i] = dr.String()
	}
	return brdgme.CommaList(parts)
}
