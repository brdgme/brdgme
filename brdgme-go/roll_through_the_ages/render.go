package roll_through_the_ages

import (
	"bytes"
	"fmt"
	"strings"
	"unicode/utf8"

	"github.com/Miniand/brdg.me/render"
)

func (g *Game) RenderName(player int) string {
	return fmt.Sprintf("{{player %d}}", player)
}

func (g *Game) PlayerRender(player int) (string, error) {
	pNum, err := g.PlayerNum(player)
	if err != nil {
		return "", err
	}
	buf := bytes.NewBuffer([]byte{})
	// Dice
	diceRow := []interface{}{}
	numberRow := []interface{}{}
	for i, d := range g.RolledDice {
		diceString := DiceStrings[d]
		diceRow = append(diceRow, render.Bold(RenderDice(d)))
		numberRow = append(numberRow, fmt.Sprintf(
			`%s{{c "gray"}}%d{{/c}}`,
			strings.Repeat(" ", utf8.RuneCountInString(diceString)/2),
			i+1,
		))
	}
	for _, d := range g.KeptDice {
		diceRow = append(diceRow, RenderDice(d))
	}
	buf.WriteString("{{b}}Dice{{/b}} {{c \"gray\"}}(F: food, W: worker, G: good, C: coin, X: skull){{/c}}\n")
	t := render.Table([][]interface{}{diceRow, numberRow}, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Remaining turns
	if g.FinalRound {
		buf.WriteString("{{b}}This is the final round{{/b}}\n\n")
	}
	// Turn resources
	switch g.Phase {
	case PhaseBuild, PhaseBuy:
		cells := [][]interface{}{
			{render.Bold("Turn supplies")},
			{render.Bold("Workers:"), g.RemainingWorkers},
			{render.Bold("Coins:"), fmt.Sprintf(
				"%d (%d including goods)",
				g.RemainingCoins,
				g.RemainingCoins+g.Boards[pNum].GoodsValue(),
			)},
		}
		buf.WriteString(render.Table(cells, 0, 2))
		buf.WriteString("\n\n")
	case PhaseTrade:
		cells := [][]interface{}{
			{render.Bold("Turn supplies")},
			{render.Bold("Ships:"), g.RemainingShips},
		}
		buf.WriteString(render.Table(cells, 0, 2))
		buf.WriteString("\n\n")
	}
	// Cities
	buf.WriteString("{{b}}Cities{{/b}} {{c \"gray\"}}(number of dice and food used per turn){{/c}}\n")
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
	cells := [][]interface{}{{"{{b}}Player{{/b}}", cityHeaderBuf.String()}}
	for p, _ := range g.Players {
		remaining := MaxCityProgress - g.Boards[p].CityProgress
		row := []interface{}{
			g.RenderName(p),
			fmt.Sprintf(
				"%s%s",
				strings.Repeat(fmt.Sprintf(
					`%s `,
					RenderX(p, p == pNum),
				), g.Boards[p].CityProgress+1),
				strings.Repeat(
					`{{c "gray"}}.{{/c}} `,
					remaining,
				),
			),
		}
		if remaining > 0 {
			row = append(row, render.Markup(
				fmt.Sprintf("(%d left)", remaining), render.Gray, p == pNum))
		}
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Developments
	header := []interface{}{render.Bold("Development")}
	for p, _ := range g.Players {
		header = append(header, g.RenderName(p))
	}
	header = append(header, []interface{}{
		render.Bold("Cost"),
		render.Bold("Pts"),
		render.Bold("Effect"),
	}...)
	cells = [][]interface{}{header}
	for _, d := range Developments {
		dv := DevelopmentValues[d]
		row := []interface{}{strings.Title(dv.Name)}
		for p, _ := range g.Players {
			cell := `{{c "gray"}}.{{/c}}`
			if g.Boards[p].Developments[d] {
				cell = RenderX(p, pNum == p)
			}
			row = append(row, render.Centred(cell))
		}
		row = append(row, []interface{}{
			fmt.Sprintf(" %d", dv.Cost),
			fmt.Sprintf(" %d", dv.Points),
			fmt.Sprintf(`{{c "gray"}}%s{{/c}}`, dv.Effect),
		}...)
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Monuments
	header = []interface{}{render.Bold("Monument")}
	for p, _ := range g.Players {
		header = append(header, g.RenderName(p))
	}
	header = append(header, []interface{}{
		render.Bold("Size"),
		render.Bold("Pts"),
		render.Bold("Effect"),
	}...)
	cells = [][]interface{}{header}
	for _, m := range Monuments {
		mv := MonumentValues[m]
		row := []interface{}{strings.Title(mv.Name)}
		for p, _ := range g.Players {
			var cell string
			switch {
			case g.Boards[p].Monuments[m] == 0:
				cell = `{{c "gray"}}.{{/c}}`
			case g.Boards[p].Monuments[m] == mv.Size:
				cell = RenderX(p, g.Boards[p].MonumentBuiltFirst[m])
			default:
				cell = fmt.Sprintf(
					`{{c "%s"}}%d{{/c}}`,
					render.PlayerColour(p),
					g.Boards[p].Monuments[m],
				)
			}
			row = append(row, render.Centred(cell))
		}
		row = append(row, []interface{}{
			fmt.Sprintf(" %d", mv.Size),
			fmt.Sprintf("{{b}}%d{{/b}}/%d", mv.Points, mv.SubsequentPoints),
			fmt.Sprintf(`{{c "gray"}}%s{{/c}}`, mv.Effect),
		}...)
		cells = append(cells, row)
	}
	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	// Resources
	header = []interface{}{render.Bold("Resource")}
	for p, _ := range g.Players {
		header = append(header, g.RenderName(p))
	}
	cells = [][]interface{}{header}
	for _, good := range GoodsReversed() {
		row := []interface{}{RenderGoodName(good)}
		for p, _ := range g.Players {
			num := g.Boards[p].Goods[good]
			cell := render.Colour(".", "gray")
			if num > 0 {
				cell = render.Markup(
					fmt.Sprintf("%d (%d)", num, GoodValue(good, num)),
					render.PlayerColour(p),
					p == pNum,
				)
			}
			row = append(row, render.Centred(cell))
		}
		cells = append(cells, row)
	}
	row := []interface{}{render.Bold("total")}
	for p, _ := range g.Players {
		cell := render.Markup(
			fmt.Sprintf("%d (%d)", g.Boards[p].GoodsNum(), g.Boards[p].GoodsValue()),
			render.PlayerColour(p),
			p == pNum,
		)
		row = append(row, render.Centred(cell))
	}
	cells = append(cells, row, []interface{}{})

	row = []interface{}{FoodName}
	for p, _ := range g.Players {
		cell := render.Markup(
			g.Boards[p].Food,
			render.PlayerColour(p),
			p == pNum,
		)
		row = append(row, render.Centred(cell))
	}
	cells = append(cells, row)
	row = []interface{}{ShipName}
	for p, _ := range g.Players {
		cell := render.Markup(
			g.Boards[p].Ships,
			render.PlayerColour(p),
			p == pNum,
		)
		row = append(row, render.Centred(cell))
	}
	cells = append(cells, row)
	row = []interface{}{DisasterName}
	for p, _ := range g.Players {
		cell := render.Markup(
			g.Boards[p].Disasters,
			render.PlayerColour(p),
			p == pNum,
		)
		row = append(row, render.Centred(cell))
	}
	cells = append(cells, row)
	row = []interface{}{render.Bold("score")}
	for p, _ := range g.Players {
		cell := render.Markup(
			g.Boards[p].Score(),
			render.PlayerColour(p),
			p == pNum,
		)
		row = append(row, render.Centred(cell))
	}
	cells = append(cells, row)

	t = render.Table(cells, 0, 2)
	buf.WriteString(t)
	return buf.String(), nil
}

func RenderX(player int, strong bool) string {
	x := "x"
	if strong {
		x = "X"
	}
	return render.Markup(x, render.PlayerColour(player), strong)
}

func RenderDice(dice int) string {
	diceString := DiceStrings[dice]
	for v, col := range DiceValueColours {
		diceString = strings.Replace(diceString, v, fmt.Sprintf(
			`{{c "%s"}}%s{{/c}}`,
			col,
			v,
		), -1)
	}
	return diceString
}

func RenderGoodName(good Good) string {
	return fmt.Sprintf(
		`{{b}}{{c "%s"}}%s{{/c}}{{/b}}`,
		GoodColours[good],
		GoodStrings[good],
	)
}

var FoodName = `{{b}}{{c "green"}}food{{/c}}{{/b}}`
var ShipName = `{{b}}{{c "blue"}}ship{{/c}}{{/b}}`
var DisasterName = `{{b}}{{c "red"}}disaster{{/c}}{{/b}}`
