package sushi_go

import (
	"bytes"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	CardColumnTempura = iota
	CardColumnSashimi
	CardColumnDumpling
	CardColumnMaki
	CardColumnNigiri
	CardColumnPudding
	CardColumnChopsticks
)

var CardColumns = []int{
	CardColumnTempura,
	CardColumnSashimi,
	CardColumnDumpling,
	CardColumnMaki,
	CardColumnNigiri,
	CardColumnPudding,
	CardColumnChopsticks,
}

var CardColumnMap = map[int]int{
	CardTempura:      CardColumnTempura,
	CardSashimi:      CardColumnSashimi,
	CardDumpling:     CardColumnDumpling,
	CardMakiRoll3:    CardColumnMaki,
	CardMakiRoll2:    CardColumnMaki,
	CardMakiRoll1:    CardColumnMaki,
	CardSalmonNigiri: CardColumnNigiri,
	CardSquidNigiri:  CardColumnNigiri,
	CardEggNigiri:    CardColumnNigiri,
	CardPudding:      CardColumnPudding,
	CardWasabi:       CardColumnNigiri,
	CardChopsticks:   CardColumnChopsticks,
}

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func (g *Game) PlayerRender(pNum int) string {
	buf := bytes.Buffer{}
	buf.WriteString(fmt.Sprintf(
		"It is round %s of %s\n\n",
		render.Bold(strconv.Itoa(g.Round)),
		render.Bold("3"),
	))
	if pNum >= 0 {
		buf.WriteString(render.Bold("Hand:\n\n"))
		explained := map[int]bool{}
		cells := [][]render.Cell{}
		for i, c := range g.Hands[pNum] {
			row := []render.Cell{
				render.Cel(render.Fg(render.Grey, fmt.Sprintf("(%d)", i+1))),
				render.Cel(RenderCard(c)),
			}
			if !explained[c] && CardExplanations[c] != "" {
				row = append(row, render.Cel(render.Fg(
					render.Grey,
					"  "+CardExplanations[c],
				)))
				explained[c] = true
				// Only explain for the first maki roll
				if c == CardMakiRoll1 || c == CardMakiRoll2 || c == CardMakiRoll3 {
					explained[CardMakiRoll1] = true
					explained[CardMakiRoll2] = true
					explained[CardMakiRoll3] = true
				}
			}
			cells = append(cells, row)
		}
		buf.WriteString(render.Table(cells, 0, 2))
		buf.WriteString("\n\n")

		playingOutput := false
		if g.Playing[pNum] != nil {
			buf.WriteString(fmt.Sprintf(
				"Playing: %s\n",
				brdgme.CommaList(RenderCards(g.Playing[pNum])),
			))
			playingOutput = true
		}
		if pNum == g.Controller && g.Playing[Dummy] != nil && g.Players == 2 {
			buf.WriteString(fmt.Sprintf(
				"Dummy:   %s\n",
				brdgme.CommaList(RenderCards(g.Playing[Dummy])),
			))
			playingOutput = true
		}
		if playingOutput {
			buf.WriteString("\n")
		}
	}

	pCount := g.AllPlayers
	dir := 1
	if g.Round%2 == 1 {
		dir = -1
	}
	for i := 0; i < pCount; i++ {
		p := pNum + i*dir
		if p < 0 {
			p += pCount
		}
		p = p % pCount
		heading := ""
		if pNum >= 0 && i == 1 {
			heading = "You are passing cards to "
		}
		buf.WriteString(g.RenderPlayerCards(p, heading))
		buf.WriteString("\n\n")
	}
	return buf.String()
}

func (g *Game) RenderPlayerCards(player int, heading string) string {
	buf := bytes.Buffer{}
	buf.WriteString(fmt.Sprintf(
		"%s%s (%s points):\n",
		heading,
		g.RenderName(player),
		render.Bold(strconv.Itoa(g.PlayerPoints[player])),
	))
	buf.WriteString(render.Table(CardsCells(g.Played[player]), 0, 3))
	return buf.String()
}

func CardsCells(cards []int) [][]render.Cell {
	columns := map[int][]string{}
	for _, c := range CardColumns {
		columns[c] = []string{}
	}
	unusedWasabi := 0
	for _, c := range cards {
		col := CardColumnMap[c]
		switch c {
		case CardWasabi:
			columns[col] = append(columns[col], RenderCard(c))
			unusedWasabi++
		case CardSalmonNigiri, CardSquidNigiri, CardEggNigiri:
			if unusedWasabi > 0 {
				columns[col][len(columns[col])-unusedWasabi] = fmt.Sprintf(
					"%s + %s",
					RenderCard(c),
					RenderCard(CardWasabi),
				)
				unusedWasabi--
			} else {
				columns[col] = append(columns[col], RenderCard(c))
			}
		default:
			columns[col] = append(columns[col], RenderCard(c))
		}
	}
	cells := [][]render.Cell{}
	y := 0
	for {
		row := []render.Cell{}
		hadContent := false
		for _, col := range CardColumns {
			l := len(columns[col])
			if l == 0 {
				// Skip empty columns
				continue
			}
			cell := render.Cel("")
			if l > y {
				cell = render.Cel(columns[col][y])
				hadContent = true
			}
			row = append(row, cell)
		}
		if !hadContent {
			break
		}
		cells = append(cells, row)
		y++
	}
	return cells
}

func RenderCard(c int) string {
	return render.Markup(CardStrings[c], CardColours[c], c != CardPlayed)
}

func RenderCards(cards []int) []string {
	cardStrs := make([]string, len(cards))
	for i, c := range cards {
		cardStrs[i] = RenderCard(c)
	}
	return cardStrs
}
