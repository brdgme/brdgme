package roll_through_the_ages

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) CanNext(player int) bool {
	return player == g.CurrentPlayer && Contains(g.Phase, []interface{}{
		PhasePreserve,
		PhaseRoll,
		PhaseExtraRoll,
		PhaseInvade,
		PhaseBuild,
		PhaseTrade,
		PhaseBuy,
	})
}

func (g *Game) Next(player int) ([]brdgme.Log, error) {
	if !g.CanNext(player) {
		return nil, errors.New("you can't next at the moment")
	}
	return g.NextPhase(), nil
}

func (g *Game) NextCommand(
	player int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Next(player)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, err
}
