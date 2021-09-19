package for_sale_1

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) PlayerRender(player int) string {
	return g.Render(&player)
}

func (g *Game) Render(player *int) string {
	output := bytes.NewBuffer([]byte{})
	switch g.CurrentPhase() {
	case BuyingPhase:
		output.WriteString(fmt.Sprintf("Buildings available: %s\n",
			strings.Join(RenderCards(g.OpenCards, RenderBuilding), " ")))
		currentBidText := render.Fg(render.Grey, "none")
		if highestPlayer, highestAmount := g.HighestBid(); highestAmount > 0 {
			currentBidText = fmt.Sprintf(
				"%s by %s",
				render.Bold(strconv.Itoa(highestAmount)),
				render.Player(highestPlayer),
			)
		}
		output.WriteString(fmt.Sprintf(
			"Current bid: %s\n",
			currentBidText,
		))
		if player != nil {
			output.WriteString(fmt.Sprintf(
				"Your bid: %s\n",
				render.Bold(strconv.Itoa(g.Bids[*player])),
			))
		}
		remainingPlayers := []string{}
		for p := 0; p < g.Players; p++ {
			if !g.FinishedBidding[p] {
				remainingPlayers = append(remainingPlayers, render.Player(p))
			}
		}
		output.WriteString(fmt.Sprintf(
			"Remaining players: %s\n\n",
			brdgme.CommaList(remainingPlayers),
		))
	case SellingPhase:
		output.WriteString(fmt.Sprintf("Cheques available: %s\n",
			strings.Join(RenderCards(g.OpenCards, RenderCheque), " ")))
		if player != nil && g.Bids[*player] != 0 {
			output.WriteString(fmt.Sprintf(
				"You are playing: %s\n",
				RenderBuilding(g.Bids[*player]),
			))
		}
		output.WriteString("\n")
	}
	if player != nil {
		output.WriteString(fmt.Sprintf(
			"Your chips: %s\n",
			render.Bold(strconv.Itoa(g.Chips[*player])),
		))
		output.WriteString(fmt.Sprintf(
			"Your buildings: %s\n",
			strings.Join(RenderCards(g.Hands[*player], RenderBuilding), " "),
		))
		output.WriteString(fmt.Sprintf(
			"Your cheques: %s",
			strings.Join(RenderCards(g.Cheques[*player], RenderCheque), " "),
		))
	}

	if !g.IsFinished() {
		var (
			rounds    int
			roundType string
		)
		switch g.CurrentPhase() {
		case BuyingPhase:
			rounds = (g.BuildingDeck.Len() / g.Players) + 1
			roundType = "buying"
		case SellingPhase:
			rounds = (g.ChequeDeck.Len() / g.Players) + 1
			roundType = "selling"
		}
		output.WriteString(fmt.Sprintf(
			"\n\n%s %s %s remaining",
			render.Bold(strconv.Itoa(rounds)),
			roundType,
			brdgme.Plural(rounds, "round"),
		))
	}
	return output.String()

}

func (g *Game) PubRender() string {
	return g.Render(nil)
}
