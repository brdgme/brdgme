package age_of_war

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme-go/render"
)

func (g *Game) PlayerRender(player int) string {
	return g.PubRender()
}

func (g *Game) PubRender() string {
	layout := []string{
		render.Bold("Current roll"),
		strings.Join(RenderDice(g.CurrentRoll), "   "),
		"",
	}

	if g.CurrentlyAttacking != -1 {
		layout = append(layout, []string{
			render.Bold("Currently attacking"),
			"",
			g.RenderCastle(
				g.CurrentlyAttacking,
				g.CurrentRoll,
			),
			"",
		}...)
	}

	layout = append(layout, []string{
		"",
		render.Bold("Castles"),
		"",
	}...)
	layout = append(layout, g.RenderCastles())

	scores := g.Scores()
	scoreStrs := make([]string, g.Players)
	for p := 0; p < g.Players; p++ {
		scoreStrs[p] = fmt.Sprintf(
			"%s: %s",
			render.Player(p),
			render.Bold(strconv.Itoa(scores[p])),
		)
	}
	layout = append(layout, []string{
		"",
		render.Bold("Scores"),
		strings.Join(scoreStrs, "   "),
	}...)

	return render.Layout(layout)
}

func (g *Game) RenderCastles() string {
	cells := [][]render.Cell{}
	row := []render.Cell{}
	lastClan := -1
	conqueredClans := map[int]bool{}
	for i, c := range Castles {
		if lastClan != -1 && c.Clan != lastClan && len(row) > 0 {
			cells = append(cells, []render.Cell{
				render.Cel(
					render.Table(
						[][]render.Cell{row}, 0, 6), render.Center)})
			row = []render.Cell{}
		}
		conquered, ok := conqueredClans[c.Clan]
		if !ok {
			var conqueredBy int
			conquered, conqueredBy = g.ClanConquered(c.Clan)
			conqueredClans[c.Clan] = conquered
			if conquered {
				cells = append(cells, []render.Cell{render.Cel(
					fmt.Sprintf(
						"%s has been conquered by %s for %s points",
						RenderClan(c.Clan),
						render.Player(conqueredBy),
						render.Bold(strconv.Itoa(ClanSetPoints[c.Clan])),
					), render.Center),
				})
			}
		}
		if !conquered {
			row = append(row, render.Cel(
				g.RenderCastle(i, g.CurrentRoll),
				render.Center,
			))
		}
		lastClan = c.Clan
	}
	if len(row) > 0 {
		cells = append(cells, []render.Cell{render.Cel(
			render.Table(
				[][]render.Cell{row}, 0, 6),
			render.Center,
		)})
	}
	return render.Table(cells, 1, 6)
}

func (g *Game) RenderCastle(cIndex int, roll []int) string {
	c := Castles[cIndex]
	cells := [][]render.Cell{{
		render.Cel(
			fmt.Sprintf(
				"%s (%d)",
				c.RenderName(),
				c.Points,
			),
			render.Center,
		)},
	}
	if g.Conquered[cIndex] {
		cells = append(cells, []render.Cell{
			render.Cel(
				fmt.Sprintf(
					"(%s)",
					render.Player(g.CastleOwners[cIndex]),
				),
				render.Center,
			)})
	}
	for i, l := range c.CalcLines(g.Conquered[cIndex]) {
		row := []render.Cell{render.Cel(
			render.Fg(render.Grey, fmt.Sprintf(
				"%d.",
				i+1,
			)),
		)}
		canAfford, _ := l.CanAfford(g.CurrentRoll)
		if (g.CurrentlyAttacking == cIndex || g.CurrentlyAttacking == -1) &&
			(!g.Conquered[cIndex] || g.CastleOwners[cIndex] != g.CurrentPlayer) &&
			!g.CompletedLines[i] && canAfford {
			row = append(row, render.Cel(
				render.Bold(render.Fg(render.Green, "X ")),
			))
		} else {
			row = append(row, render.Cel(
				"  ",
			))
		}
		if g.CurrentlyAttacking == cIndex && g.CompletedLines[i] {
			row = append(row, render.Cel(render.Fg(render.Grey, "complete")))
		} else {
			row = append(row, l.RenderRow()...)
		}
		cells = append(cells, []render.Cell{
			render.Cel(render.Table([][]render.Cell{row}, 0, 1)),
		})
	}
	return render.Table(cells, 0, 0)
}

func RenderDie(die int) string {
	return render.Bold(render.Fg(DiceColours[die], DiceStrings[die]))
}

func RenderDice(dice []int) []string {
	l := len(dice)
	if l == 0 {
		return []string{}
	}
	strs := make([]string, l)
	for i, d := range dice {
		strs[i] = RenderDie(d)
	}
	return strs
}

func RenderInf(n int) string {
	return render.Bold(render.Fg(InfantryColour, fmt.Sprintf("%d inf", n)))
}

func (c Castle) RenderName() string {
	return render.Bold(render.Fg(ClanColours[c.Clan], c.Name))
}

func RenderClan(clan int) string {
	return render.Bold(render.Fg(ClanColours[clan], ClanNames[clan]))
}

func (l Line) RenderRow() []render.Cell {
	row := []render.Cell{}
	for _, s := range l.Symbols {
		row = append(row, render.Cel(RenderDie(s)))
	}
	if l.Infantry > 0 {
		row = append(row, render.Cel(RenderInf(l.Infantry)))
	}
	return row
}

func (l Line) String() string {
	return render.Table([][]render.Cell{l.RenderRow()}, 0, 2)
}
