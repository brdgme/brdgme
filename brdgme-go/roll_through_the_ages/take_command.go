package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type TakeAction int

const (
	TakeFood TakeAction = iota
	TakeWorkers
)

var TakeMap = map[TakeAction]string{
	TakeFood:    "food",
	TakeWorkers: "workers",
}

func (g *Game) CanTake(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseCollect
}

func (g *Game) Take(player int, actions []TakeAction) ([]brdgme.Log, error) {
	if !g.CanTake(player) {
		return nil, errors.New("you can't take at the moment")
	}
	numDice := 0
	for _, d := range g.KeptDice {
		if d == DiceFoodOrWorkers {
			numDice += 1
		}
	}
	if l := len(actions); l != numDice {
		return nil, fmt.Errorf(
			"you must specify %d take actions after the take command",
			l,
		)
	}

	cp := g.CurrentPlayer
	for _, a := range actions {
		switch a {
		case TakeFood:
			g.Boards[cp].Food += 2 + g.Boards[cp].FoodModifier()
		case TakeWorkers:
			g.RemainingWorkers += 2 + g.Boards[cp].WorkerModifier()
		default:
			return nil, errors.New("could not understand action")
		}
	}

	return g.NextPhase(), nil
}

func (g *Game) TakeCommand(
	player int,
	actions []TakeAction,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Take(player, actions)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, nil
}
