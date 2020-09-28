package farkle

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type DoneCommand struct{}

func (dc DoneCommand) Name() string { return "done" }

func (dc DoneCommand) Call(
	player string,
	context interface{},
	input *command.Reader,
) (string, error) {
	g := context.(*Game)
	pNum, ok := g.PlayerNum(player)
	if !ok {
		return "", errors.New("cannot find player")
	}
	if !g.CanDone(pNum) {
		return "", errors.New("can't call done at the moment")
	}
	g.Log.Add(log.NewPublicMessage(fmt.Sprintf(
		"%s took {{b}}%d{{_b}} points, now on {{b}}%d{{_b}}",
		render.PlayerName(g.Player, g.Players[g.Player]),
		g.TurnScore,
		g.Scores[pNum]+g.TurnScore,
	)))
	g.Scores[pNum] = g.Scores[pNum] + g.TurnScore
	g.NextPlayer()
	return "", nil
}

func (dc DoneCommand) Usage(player string, context interface{}) string {
	return "{{b}}done{{_b}} to take the points and finish your turn"
}
*/

func (g *Game) Done(pNum int) ([]brdgme.Log, error) {
	if !g.CanDone(pNum) {
		return nil, errors.New("can't call done at the moment")
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s took %s points, now on %s",
		render.Player(g.Player),
		render.Bold(strconv.Itoa(g.TurnScore)),
		render.Bold(strconv.Itoa(g.Scores[pNum]+g.TurnScore)),
	))}
	g.Scores[pNum] = g.Scores[pNum] + g.TurnScore
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanDone(player int) bool {
	return player == g.Player && g.TakenThisRoll && !g.IsFinished()
}
