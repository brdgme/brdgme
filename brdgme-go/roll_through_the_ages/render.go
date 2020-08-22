package roll_through_the_ages

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) RenderName(player int) string {
	return fmt.Sprintf("{{player %d}}", player)
}

func (g *Game) PlayerRender(player int) string {
	buf := bytes.NewBuffer([]byte{})
	// Dice
	diceRow := []render.Cell{}
	numberRow := []render.Cell{}
	for i, d := range g.RolledDice {
		diceString := DiceStrings[d]
		diceRow = append(diceRow, render.Cel(render.Bold(RenderDice(d))))
		numberRow = append(numberRow, render.Cel(fmt.Sprintf(
			`%s%v`,
			strings.Repeat(" ", len(diceString)/2),
			render.Fg(render.Grey, strconv.Itoa(i+1)),
		)))
	}
	for _, d := range g.KeptDice {
		diceRow = append(diceRow, render.Cell{Align: render.Left, Content: RenderDice(d)})
	}
	buf.WriteString("{{b}}Dice{{/b}} ")
	buf.WriteString(render.Fg(render.Grey, "(F: food, W: worker, G: good, C: coin, X: skull)"))
	buf.WriteString("\n")
	t := render.Table([][]render.Cell{diceRow, numberRow}, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Remaining turns
	if g.FinalRound {
		buf.WriteString("{{b}}This is the final round{{/b}}\n\n")
	}
	// Turn resources
	switch g.Phase {
	case PhaseBuild, PhaseBuy:
		cells := [][]render.Cell{
			{render.Cel(render.Bold("Turn supplies"))},
			{render.Cel(render.Bold("Workers:")), render.Cel(strconv.Itoa(g.RemainingWorkers))},
			{render.Cel(render.Bold("Coins:")), render.Cel(fmt.Sprintf(
				"%d (%d including goods)",
				g.RemainingCoins,
				g.RemainingCoins+g.Boards[player].GoodsValue(),
			))},
		}
		buf.WriteString(render.Table(cells, 0, 2))
		buf.WriteString("\n\n")
	case PhaseTrade:
		cells := [][]render.Cell{
			{render.Cel(render.Bold("Turn supplies"))},
			{render.Cel(render.Bold("Ships:")), render.Cel(strconv.Itoa(g.RemainingShips))},
		}
		buf.WriteString(render.Table(cells, 0, 2))
		buf.WriteString("\n\n")
	}
	// Cities
	buf.WriteString("{{b}}Cities{{/b}} ")
	buf.WriteString(render.Fg(render.Grey, "(number of dice and food used per turn)"))
	buf.WriteString("\n")
	cityHeaderBuf := bytes.NewBufferString(fmt.Sprintf(
		"{{b}}%d{{/b}}", BaseCitySize))
	last := 0
	for i, n := range CityLevels {
		cityHeaderBuf.WriteString(fmt.Sprintf(
			`%s{{b}}%d{{/b}}`,
			strings.Repeat(" ", (n-last-1)*2+1),
			BaseCitySize+i+1,
		))
		last = n
	}
	cells := [][]render.Cell{{render.Cel("{{b}}Player{{/b}}"), render.Cel(cityHeaderBuf.String())}}
	for p := 0; p < g.PlayerCount(); p++ {
		remaining := MaxCityProgress - g.Boards[p].CityProgress
		row := []render.Cell{
			render.Cel(g.RenderName(p)),
			render.Cel(fmt.Sprintf(
				"%s%s",
				strings.Repeat(fmt.Sprintf(
					`%s `,
					RenderX(p, p == player),
				), g.Boards[p].CityProgress+1),
				strings.Repeat(
					render.Fg(render.Grey, ".")+" ",
					remaining,
				),
			)),
		}
		if remaining > 0 {
			row = append(row, render.Cel(render.Markup(
				fmt.Sprintf("(%d left)", remaining), render.Grey, p == player)))
		}
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Developments
	header := []render.Cell{render.Cel(render.Bold("Development"))}
	for p := 0; p < g.PlayerCount(); p++ {
		header = append(header, render.Cel(g.RenderName(p)))
	}
	header = append(header, []render.Cell{
		render.Cel(render.Bold("Cost")),
		render.Cel(render.Bold("Pts")),
		render.Cel(render.Bold("Effect")),
	}...)
	cells = [][]render.Cell{header}
	for _, d := range Developments {
		dv := DevelopmentValues[d]
		row := []render.Cell{render.Cel(strings.Title(dv.Name))}
		for p := 0; p < g.PlayerCount(); p++ {
			cell := render.Fg(render.Grey, ".")
			if g.Boards[p].Developments[d] {
				cell = RenderX(p, player == p)
			}
			row = append(row, render.Cel(cell, render.Center))
		}
		row = append(row, []render.Cell{
			render.Cel(fmt.Sprintf(" %d", dv.Cost)),
			render.Cel(fmt.Sprintf(" %d", dv.Points)),
			render.Cel(render.Fg(render.Grey, dv.Effect)),
		}...)
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Monuments
	header = []render.Cell{render.Cel(render.Bold("Monument"))}
	for p := 0; p < g.PlayerCount(); p++ {
		header = append(header, render.Cel(g.RenderName(p)))
	}
	header = append(header, []render.Cell{
		render.Cel(render.Bold("Size")),
		render.Cel(render.Bold("Pts")),
		render.Cel(render.Bold("Effect")),
	}...)
	cells = [][]render.Cell{header}
	for _, m := range Monuments {
		mv := MonumentValues[m]
		row := []render.Cell{render.Cel(strings.Title(mv.Name))}
		for p := 0; p < g.PlayerCount(); p++ {
			var cell string
			switch {
			case g.Boards[p].Monuments[m] == 0:
				cell = render.Fg(render.Grey, ".")
			case g.Boards[p].Monuments[m] == mv.Size:
				cell = RenderX(p, g.Boards[p].MonumentBuiltFirst[m])
			default:
				cell = render.Fgp(p, strconv.Itoa(g.Boards[p].Monuments[m]))
			}
			row = append(row, render.Cel(cell, render.Center))
		}
		row = append(row, []render.Cell{
			render.Cel(fmt.Sprintf(" %d", mv.Size)),
			render.Cel(fmt.Sprintf("{{b}}%d{{/b}}/%d", mv.Points, mv.SubsequentPoints)),
			render.Cel(render.Fg(render.Grey, mv.Effect)),
		}...)
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Resources
	header = []render.Cell{render.Cel(render.Bold("Resource"))}
	for p := 0; p < g.PlayerCount(); p++ {
		header = append(header, render.Cel(g.RenderName(p)))
	}
	cells = [][]render.Cell{header}
	for _, good := range GoodsReversed() {
		row := []render.Cell{render.Cel(RenderGoodName(good))}
		for p := 0; p < g.PlayerCount(); p++ {
			num := g.Boards[p].Goods[good]
			cell := render.Fg(render.Grey, ".")
			if num > 0 {
				cell = render.BoldIf(
					render.Fgp(p, fmt.Sprintf("%d (%d)", num, GoodValue(good, num))),
					p == player,
				)
			}
			row = append(row, render.Cel(cell, render.Center))
		}
		cells = append(cells, row)
	}
	row := []render.Cell{render.Cel(render.Bold("total"))}
	for p := 0; p < g.PlayerCount(); p++ {
		cell := render.BoldIf(
			render.Fgp(p, fmt.Sprintf("%d (%d)", g.Boards[p].GoodsNum(), g.Boards[p].GoodsValue())),
			p == player,
		)
		row = append(row, render.Cel(cell, render.Center))
	}
	cells = append(cells, row, []render.Cell{})

	row = []render.Cell{render.Cel(FoodName)}
	for p := 0; p < g.PlayerCount(); p++ {
		cell := render.BoldIf(
			render.Fgp(p, strconv.Itoa(g.Boards[p].Food)),
			p == player,
		)
		row = append(row, render.Cel(cell, render.Center))
	}
	cells = append(cells, row)
	row = []render.Cell{render.Cel(ShipName)}
	for p := 0; p < g.PlayerCount(); p++ {
		cell := render.BoldIf(
			render.Fgp(p, strconv.Itoa(g.Boards[p].Ships)),
			p == player,
		)
		row = append(row, render.Cel(cell, render.Center))
	}
	cells = append(cells, row)
	row = []render.Cell{render.Cel(DisasterName)}
	for p := 0; p < g.PlayerCount(); p++ {
		cell := render.BoldIf(
			render.Fgp(p, strconv.Itoa(g.Boards[p].Disasters)),
			p == player,
		)
		row = append(row, render.Cel(cell, render.Center))
	}
	cells = append(cells, row)
	row = []render.Cell{render.Cel(render.Bold("score"))}
	for p := 0; p < g.PlayerCount(); p++ {
		cell := render.BoldIf(
			render.Fgp(p, strconv.Itoa(g.Boards[p].Score())),
			p == player,
		)
		row = append(row, render.Cel(cell, render.Center))
	}
	cells = append(cells, row)

	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	return buf.String()
}

func (g *Game) PubRender() string {
	// No hidden information, so the public render is just the render for the
	// active player
	return g.PlayerRender(g.CurrentPlayer)
}

func RenderX(player int, strong bool) string {
	x := "x"
	if strong {
		x = "X"
	}
	return render.BoldIf(render.Fgp(player, x), strong)
}

func RenderDice(dice Die) string {
	diceString := DiceStrings[dice]
	for v, col := range DiceValueColours {
		diceString = strings.Replace(diceString, v, render.Fg(col, v), -1)
	}
	return diceString
}

func RenderGoodName(good Good) string {
	return render.Bold(render.Fg(GoodColours[good], GoodStrings[good]))
}

var FoodName = render.Bold(render.Fg(render.Green, "food"))
var ShipName = render.Bold(render.Fg(render.Blue, "ship"))
var DisasterName = render.Bold(render.Fg(render.Red, "disaster"))
