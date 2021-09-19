package farkle_1

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

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
