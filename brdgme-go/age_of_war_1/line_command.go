package age_of_war_1

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) LineCommand(
	player int,
	line int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Line(player, line-1)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CanLine(player int) bool {
	return g.CurrentPlayer == player && g.CurrentlyAttacking != -1
}

func (g *Game) Line(player, line int) ([]brdgme.Log, error) {
	if !g.CanLine(player) {
		return nil, errors.New("unable to complete a line right now")
	}
	lines := Castles[g.CurrentlyAttacking].CalcLines(
		g.Conquered[g.CurrentlyAttacking],
	)
	if line < 0 || line >= len(lines) {
		return nil, errors.New("that is not a valid line")
	}
	if g.CompletedLines[line] {
		return nil, errors.New("that line has already been completed")
	}
	canAfford, with := lines[line].CanAfford(g.CurrentRoll)
	if !canAfford {
		return nil, errors.New("cannot afford that line")
	}
	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"%s completed %s with %s %s",
			render.Player(player),
			lines[line].String(),
			render.Bold(strconv.Itoa(with)),
			brdgme.Plural(with, "die"),
		)),
	}
	g.CompletedLines[line] = true
	// Check end of turn first in case they completed the castle.
	isEndOfTurn, endOfTurnLogs := g.CheckEndOfTurn()
	logs = append(logs, endOfTurnLogs...)
	if !isEndOfTurn {
		logs = append(logs, g.Roll(len(g.CurrentRoll)-with))
		_, endOfTurnLogs = g.CheckEndOfTurn()
		logs = append(logs, endOfTurnLogs...)
	}
	return logs, nil
}
