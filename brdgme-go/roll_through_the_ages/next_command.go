package roll_through_the_ages

import (
	"errors"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

// type NextCommand struct{}

// func (c NextCommand) Name() string { return "next" }

// func (c NextCommand) Call(
// 	player string,
// 	context interface{},
// 	input *command.Reader,
// ) (string, error) {
// 	g := context.(*Game)
// 	pNum, err := g.PlayerNum(player)
// 	if err != nil {
// 		return "", err
// 	}
// 	return "", g.Next(pNum)
// }

// func (c NextCommand) Usage(player string, context interface{}) string {
// 	return "{{b}}next{{/b}} to continue to the next phase of your turn"
// }

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
