package roll_through_the_ages_1

import (
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) CanRoll(player int) bool {
	if g.CurrentPlayer != player {
		return false
	}
	return g.Phase == PhaseRoll &&
		(g.RemainingRolls > 0 && len(g.RolledDice) > 0) ||
		g.Phase == PhaseExtraRoll
}

func (g *Game) Roll(player int, diceNum []int) ([]brdgme.Log, error) {
	if !g.CanRoll(player) {
		return nil, errors.New("you can't roll at the moment")
	}
	if len(diceNum) == 0 {
		return nil, errors.New("you must specify which dice to roll")
	}
	if g.Phase == PhaseExtraRoll && len(diceNum) > 1 {
		return nil, errors.New("you may only roll one dice on the extra roll")
	}
	l := len(g.RolledDice)
	for _, n := range diceNum {
		if n < 0 || n > l {
			return nil, fmt.Errorf("dice number must be between 1 and %d", l)
		}
	}
	kept := []Die{}
	for i, d := range g.RolledDice {
		if !ContainsInt(i+1, diceNum) {
			kept = append(kept, d)
		}
	}
	rolled := RollN(len(g.RolledDice) - len(kept))
	g.RolledDice = append(rolled, kept...)
	logs := g.LogRoll(rolled, append(kept, g.KeptDice...))
	logs = append(logs, g.KeepSkulls()...)
	switch g.Phase {
	case PhaseRoll:
		g.RemainingRolls -= 1
		if g.RemainingRolls == 0 {
			logs = append(logs, g.NextPhase()...)
		}
	case PhaseExtraRoll:
		logs = append(logs, g.NextPhase()...)
	}
	return logs, nil
}

func (g *Game) NewRoll(n int) []brdgme.Log {
	g.RolledDice = RollN(n)
	logs := g.LogRoll(g.RolledDice, []Die{})
	g.KeptDice = []Die{}
	logs = append(logs, g.KeepSkulls()...)
	return logs
}

func (g *Game) KeepSkulls() []brdgme.Log {
	if g.PlayerCount() == 1 {
		// You can reroll skulls in single player
		return nil
	}
	i := 0
	for i < len(g.RolledDice) {
		switch g.RolledDice[i] {
		case DiceSkull:
			g.RolledDice = append(g.RolledDice[:i], g.RolledDice[i+1:]...)
			g.KeptDice = append(g.KeptDice, DiceSkull)
		default:
			i += 1
			continue
		}
	}
	logs := []brdgme.Log{}
	if len(g.RolledDice) == 0 && !(g.Phase == PhaseExtraRoll &&
		g.Boards[g.CurrentPlayer].Developments[DevelopmentLeadership]) {
		logs = g.NextPhase()
	}
	return logs
}

func (g *Game) LogRoll(newDice, oldDice []Die) []brdgme.Log {
	diceStrings := []string{}
	for _, d := range newDice {
		diceStrings = append(diceStrings, render.Bold(RenderDice(d)))
	}
	for _, d := range oldDice {
		diceStrings = append(diceStrings, RenderDice(d))
	}
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s rolled  %s`,
		g.RenderName(g.CurrentPlayer),
		strings.Join(diceStrings, "  "),
	))}
}

func (g *Game) RollCommand(
	player int,
	dice []int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Roll(player, dice)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, nil
}
