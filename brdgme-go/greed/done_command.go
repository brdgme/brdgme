package greed

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
	logs := []brdgme.Log{}
	// We take any remaining scoring combos
	for {
		available := AvailableScores(g.RemainingDice)
		if len(available) == 0 {
			break
		}
		scoreLogs, err := g.Score(pNum, available[0].Dice)
		if err != nil {
			// invariant
			panic(err)
		}
		logs = append(logs, scoreLogs...)
	}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s took %s points, now on %s",
		render.Player(g.Player),
		render.Bold(strconv.Itoa(g.TurnScore)),
		render.Bold(strconv.Itoa(g.Scores[pNum]+g.TurnScore)),
	)))
	g.Scores[pNum] = g.Scores[pNum] + g.TurnScore
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanDone(player int) bool {
	return player == g.Player && !g.IsFinished()
}
