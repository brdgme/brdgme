package age_of_war

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) RollCommand(
	player int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.RollForPlayer(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}

func (g *Game) CanRoll(player int) bool {
	return g.CurrentPlayer == player
}

func (g *Game) RollForPlayer(player int) ([]brdgme.Log, error) {
	if !g.CanRoll(player) {
		return nil, errors.New("unable to roll right now")
	}
	logs := []brdgme.Log{g.Roll(len(g.CurrentRoll) - 1)}
	_, endOfTurnLogs := g.CheckEndOfTurn()
	logs = append(logs, endOfTurnLogs...)
	return logs, nil
}
