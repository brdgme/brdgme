package roll_through_the_ages_1

import (
	"bytes"
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanInvade(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseInvade &&
		g.Boards[player].Developments[DevelopmentSmithing] &&
		g.Boards[player].Goods[GoodSpearhead] > 0
}

func (g *Game) Invade(player, amount int) ([]brdgme.Log, error) {
	if !g.CanInvade(player) {
		return nil, errors.New("you can't invade at the moment")
	}
	if amount <= 0 {
		return nil, errors.New("you must specify a positive amount of spearheads")
	}
	sh := g.Boards[player].Goods[GoodSpearhead]
	if amount > sh {
		return nil, fmt.Errorf("you only have %d spearheads", sh)
	}

	g.Boards[player].Goods[GoodSpearhead] -= amount
	buf := bytes.NewBufferString(fmt.Sprintf(
		`%s used {{b}}%d{{/b}} spearheads to cause extra damage`,
		g.RenderName(player),
		amount,
	))
	playerCount := g.PlayerCount()
	for p := 0; p < playerCount; p++ {
		if p == player {
			continue
		}
		if g.Boards[p].HasBuilt(MonumentGreatWall) {
			buf.WriteString(fmt.Sprintf(
				"\n  %s avoids the extra damage with their wall",
				g.RenderName(p),
			))
		} else {
			g.Boards[p].Disasters += amount * 2
			buf.WriteString(fmt.Sprintf(
				"\n  %s takes {{b}}%d disaster points{{/b}}",
				g.RenderName(p),
				amount,
			))
		}
	}

	logs := []brdgme.Log{brdgme.NewPublicLog(buf.String())}
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}

func (g *Game) InvadeCommand(
	player int,
	amount int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Invade(player, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   g.CurrentPlayer == player,
		Remaining: remaining,
	}, nil
}
