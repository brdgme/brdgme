package love_letter

import (
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
	leader := brdgme.IntMax(g.PlayerPoints...)
	rows := []string{fmt.Sprintf(
		"The leader has {{b}}%d %s{{/b}}, the game will end at {{b}}%d points{{/b}}",
		leader,
		brdgme.Plural(leader, "point"),
		endScores[g.Players],
	), ""}

	if pNum >= 0 {
		if g.Eliminated[pNum] {
			rows = append(
				rows,
				render.Bold("You have been eliminated from this round"),
			)
		} else {
			rows = append(
				rows,
				render.Bold(fmt.Sprintf(
					"Your %s",
					brdgme.Plural(len(g.Hands[pNum]), "card"),
				)),
				strings.Join(RenderCards(g.Hands[pNum]), "   "),
			)
		}
	}

	playerTable := [][]render.Cell{
		{},
		{
			render.Cel(render.Bold("Player")),
			render.Cel(render.Bold("Pts"), render.Center),
			render.Cel(render.Bold("Status"), render.Center),
			render.Cel(render.Bold("Discards"), render.Center),
		},
	}
	for p := 0; p < g.Players; p++ {
		status := render.Markup("active", render.Green, true)
		if g.Eliminated[p] {
			status = render.Markup("eliminated", render.Grey, false)
		} else if g.Protected[p] {
			status = render.Markup("protected", render.Black, true)
		}
		playerTable = append(playerTable, []render.Cell{
			render.Cel(render.Player(p)),
			render.Cel(render.Bold(strconv.Itoa(g.PlayerPoints[p])), render.Center),
			render.Cel(status, render.Center),
			render.Cel(strings.Join(RenderCards(g.Discards[p]), "  ")),
		})
	}
	rows = append(rows,
		render.Table(playerTable, 0, 2),
		"",
		render.Bold(fmt.Sprintf("Cards remaining: %d", len(g.Deck))),
	)

	helpTable := [][]render.Cell{
		{},
		{},
		{
			render.Cel(render.Bold("Card")),
			render.Cel(render.Bold("#")),
			render.Cel(render.Bold("Description")),
		},
	}
	for c := Princess; c >= Guard; c-- {
		helpTable = append(helpTable, []render.Cell{
			render.Cel(RenderCard(c)),
			render.Cel(strconv.Itoa(brdgme.IntCount(c, Deck))),
			render.Cel(render.Fg(render.Grey, Cards[c].Text)),
		})
	}
	rows = append(rows, render.Table(helpTable, 0, 2))

	return render.Layout(rows)
}

func RenderCard(card int) string {
	return render.Markup(fmt.Sprintf(
		"%s (%d)",
		Cards[card].Name,
		Cards[card].Number,
	), Cards[card].Color, true)
}

func RenderCards(cards []int) []string {
	strs := make([]string, len(cards))
	for i, c := range cards {
		strs[i] = RenderCard(c)
	}
	return strs
}
