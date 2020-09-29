package farkle

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/die"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) Score(player int, dice []int) ([]brdgme.Log, error) {
	// Check that it's a valid value string and get the points
	score := 0
	for _, s := range Scores() {
		if die.DiceEquals(dice, s.Dice) {
			score = s.Value
			break
		}
	}
	if score == 0 {
		return nil, errors.New(
			"That doesn't score any points",
		)
	}
	// Check that we've actually got the dice
	isIn, remaining := die.DiceInDice(dice, g.RemainingDice)
	if !isIn {
		return nil, errors.New("You don't have those dice")
	}
	g.TurnScore += score
	g.TakenThisRoll = true
	g.RemainingDice = remaining
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s scored %s for %s points",
		render.Player(g.Player),
		RenderDice(dice),
		render.Bold(strconv.Itoa(score)),
	))}, nil
}

func (g *Game) CanScore(player int) bool {
	return player == g.Player && !g.IsFinished() &&
		len(AvailableScores(g.RemainingDice)) > 0
}
