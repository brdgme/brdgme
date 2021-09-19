package splendor_1

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/libcost"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var ResourceColours = map[int]render.Color{
	Diamond:  render.Black,
	Sapphire: render.Blue,
	Emerald:  render.Green,
	Ruby:     render.Red,
	Onyx:     render.Grey,
	Gold:     render.Yellow,
	Prestige: render.Purple,
}

var ResourceStrings = map[int]string{
	Diamond:  "Diamond",
	Sapphire: "Sapphire",
	Emerald:  "Emerald",
	Ruby:     "Ruby",
	Onyx:     "Onyx",
	Gold:     "Gold",
	Prestige: "Prestige",
}

var ResourceAbbr = map[int]string{
	Diamond:  "Diam",
	Sapphire: "Saph",
	Emerald:  "Emer",
	Ruby:     "Ruby",
	Onyx:     "Onyx",
	Gold:     "Gold",
	Prestige: "VP",
}

func GemStrings() map[int]string {
	strs := map[int]string{}
	for _, g := range Gems {
		strs[g] = ResourceStrings[g]
	}
	return strs
}

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func (g *Game) PlayerRender(pNum int) string {
	var (
		pb      *PlayerBoard
		bonuses *libcost.Cost
	)
	if pNum >= 0 {
		pb = &g.PlayerBoards[pNum]
		b := pb.Bonuses()
		bonuses = &b
	}

	output := bytes.NewBuffer([]byte{})

	// Nobles
	nobleHeader := []render.Cell{render.Cel("")}
	nobleRow := []render.Cell{render.Cel(render.Fg(
		render.Grey,
		fmt.Sprintf(
			"Nobles (%s each)",
			render.Bold(RenderResourceColour("3", Prestige)),
		),
	))}
	for i, n := range g.Nobles {
		nobleHeader = append(nobleHeader, render.Cel(
			render.Fg(render.Grey, strconv.Itoa(i+1)),
			render.Center,
		))
		nobleRow = append(nobleRow, render.Cel(RenderAmount(n.Cost)))
	}
	table := [][]render.Cell{
		nobleHeader,
		nobleRow,
	}
	output.WriteString(render.Table(table, 0, 2))
	output.WriteString("\n\n")

	// Board
	longestRow := 0
	for _, r := range g.Board {
		if l := len(r); l > longestRow {
			longestRow = l
		}
	}
	if pNum >= 0 {
		if l := len(pb.Reserve); l > longestRow {
			longestRow = l
		}
	}
	header := []render.Cell{render.Cel("")}
	for i := 0; i < longestRow; i++ {
		header = append(header, render.Cel(
			render.Markup(fmt.Sprintf("%c", 'A'+i), render.Grey, true),
			render.Center,
		))
	}
	table = [][]render.Cell{header}
	for l, r := range g.Board {
		upper := []render.Cell{
			render.Cel(render.Fg(
				render.Grey,
				fmt.Sprintf("Level {{b}}%d{{/b}}", l+1),
			)),
		}
		lower := []render.Cell{render.Cel("")}
		for _, c := range r {
			upperBuf := bytes.NewBuffer([]byte{})
			if pNum >= 0 {
				if CanAfford(*bonuses, c.Cost) {
					upperBuf.WriteString(render.Markup("X ", render.Green, true))
				} else if pb.CanAfford(c.Cost) {
					upperBuf.WriteString(render.Markup("X ", render.Yellow, true))
				}
			}
			upperBuf.WriteString(RenderCardBonusVP(c))
			upper = append(upper, render.Cel(upperBuf.String(), render.Center))
			lower = append(lower, render.Cel(RenderAmount(c.Cost), render.Center))
		}
		table = append(table, upper, lower, []render.Cell{})
	}
	upper := []render.Cell{
		render.Cel(render.Fg(render.Grey, "Level {{b}}4{{/b}}")),
	}
	lower := []render.Cell{
		render.Cel(render.Fg(render.Grey, "Reserved")),
	}
	if pNum >= 0 {
		for _, c := range pb.Reserve {
			upperBuf := bytes.NewBuffer([]byte{})
			if CanAfford(*bonuses, c.Cost) {
				upperBuf.WriteString(render.Markup("X ", render.Green, true))
			} else if pb.CanAfford(c.Cost) {
				upperBuf.WriteString(render.Markup("X ", render.Yellow, true))
			}
			upperBuf.WriteString(RenderCardBonusVP(c))
			upper = append(upper, render.Cel(upperBuf.String(), render.Center))
			lower = append(lower, render.Cel(RenderAmount(c.Cost), render.Center))
		}
	}
	table = append(table, upper, lower)
	output.WriteString(render.Table(table, 0, 3))
	output.WriteString("\n\n\n")

	// Tokens
	tableHeader := []render.Cell{render.Cel("")}
	availTokenRow := []render.Cell{render.Cel(render.Bold("Tokens left"))}
	yourTokenRow := []render.Cell{render.Cel(render.Bold("You have"))}
	yourTokenDescRow := []render.Cell{render.Cel(
		render.Markup("(card+token)", render.Grey, true),
	)}
	for _, gem := range append(Gems, Gold) {
		tableHeader = append(tableHeader, render.Cel(render.Bold(
			RenderResourceColour(ResourceAbbr[gem], gem)), render.Center))

		if pNum >= 0 {
			var yourTokenDescCell string
			yourTokenRow = append(yourTokenRow, render.Cel(render.Bold(
				strconv.Itoa((*bonuses)[gem]+pb.Tokens[gem])), render.Center))
			if gem != Gold {
				yourTokenDescCell = render.Fg(render.Grey, fmt.Sprintf(
					"(%d+%d)",
					(*bonuses)[gem],
					pb.Tokens[gem],
				))
			}
			yourTokenDescRow = append(yourTokenDescRow,
				render.Cel(yourTokenDescCell, render.Center))
		}
		availTokenRow = append(availTokenRow,
			render.Cel(strconv.Itoa(g.Tokens[gem]), render.Center))
	}
	table = [][]render.Cell{
		tableHeader,
	}
	if pNum >= 0 {
		table = append(table, yourTokenRow, yourTokenDescRow)
	}
	table = append(table, availTokenRow)
	output.WriteString(render.Table(table, 0, 3))
	output.WriteString("\n\n\n")

	// Player table
	header = []render.Cell{render.Cel("")}
	for _, gem := range Gems {
		header = append(header, render.Cel(render.Bold(
			RenderResourceColour(ResourceAbbr[gem], gem)), render.Center))
	}
	header = append(
		header,
		render.Cel(render.Bold(
			RenderResourceColour(ResourceAbbr[Gold], Gold)), render.Center),
		render.Cel(render.Bold("Tok"), render.Center),
		render.Cel(render.Bold(
			render.Fg(render.Cyan, "Res")), render.Center),
		render.Cel(render.Bold(
			RenderResourceColour(ResourceAbbr[Prestige], Prestige)), render.Center),
		render.Cel(render.Bold("Dev"), render.Center),
	)
	table = [][]render.Cell{header}
	for p := 0; p < g.Players; p++ {
		bold := p == pNum
		pb := g.PlayerBoards[p]
		bonuses := pb.Bonuses()
		row := []render.Cell{render.Cel(render.Player(p))}
		for _, gem := range Gems {
			row = append(row, render.Cel(render.BoldIf(fmt.Sprintf(
				"%d+%d",
				bonuses[gem],
				pb.Tokens[gem],
			), bold), render.Center))
		}
		row = append(
			row,
			render.Cel(render.BoldIf(strconv.Itoa(pb.Tokens[Gold]), bold), render.Center),
			render.Cel(render.BoldIf(strconv.Itoa(pb.Tokens.Sum()), bold), render.Center),
			render.Cel(render.BoldIf(strconv.Itoa(len(pb.Reserve)), bold), render.Center),
			render.Cel(render.BoldIf(strconv.Itoa(pb.Prestige()), bold), render.Center),
			render.Cel(render.BoldIf(strconv.Itoa(len(pb.Cards)), bold), render.Center),
		)
		table = append(table, row)
	}
	output.WriteString(render.Table(table, 0, 2))

	return output.String()
}

func RenderResourceColour(v string, r int) string {
	return render.Fg(ResourceColours[r], v)
}

func RenderCardBonusVP(c Card) string {
	parts := []string{
		RenderResourceColour(ResourceAbbr[c.Resource], c.Resource),
	}
	if c.Prestige > 0 {
		parts = append(parts, RenderResourceColour(strconv.Itoa(c.Prestige), Prestige))
	}
	return render.Bold(strings.Join(parts, " "))
}

func RenderAmount(a libcost.Cost) string {
	parts := []string{}
	for _, r := range Resources {
		if a[r] > 0 {
			parts = append(parts, render.Bold(RenderResourceColour(strconv.Itoa(a[r]), r)))
		}
	}
	return strings.Join(parts, render.Fg(render.Grey, "-"))
}

func RenderNobleHeader(n Noble) string {
	return RenderResourceColour(strconv.Itoa(n.Prestige), Prestige)
}

func RenderCard(c Card) string {
	return fmt.Sprintf(
		"%s (%s)",
		RenderCardBonusVP(c),
		RenderAmount(c.Cost),
	)
}

func RenderNoble(n Noble) string {
	return render.Bold(fmt.Sprintf(
		"%s (%s)",
		RenderNobleHeader(n),
		RenderAmount(n.Cost),
	))
}
