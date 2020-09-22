package zombie_dice

import (
	"errors"
	"fmt"
	"math/rand"
	"strconv"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

var rnd = rand.New(rand.NewSource(time.Now().UnixNano()))

type Game struct {
	Players int

	CurrentTurn    int
	Scores         []int
	Cup            []Dice
	RollOffPlayers map[int]bool
	Finished       bool
	CurrentRoll    DiceResultList
	Kept           DiceResultList
	RoundBrains    int
	RoundShotguns  int
}

var _ brdgme.Gamer = &Game{}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 {
		return nil, errors.New("requires at least 2 players")
	}

	g.Players = players

	g.Scores = make([]int, players)
	return g.StartTurn(), nil
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6, 7, 8}
}

func (g *Game) PlayerState(player int) interface{} {
	return nil
}

func (g *Game) PubState() interface{} {
	return nil
}

func (g *Game) Points() []float32 {
	points := make([]float32, g.Players)
	for p := 0; p < g.Players; p++ {
		points[p] = float32(g.Scores[p])
	}
	return points
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{g.Scores[p]}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		return brdgme.StatusFinished{
			Placings: g.Placings(),
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn: g.WhoseTurn(),
	}.ToStatus()
}

func (g *Game) ShakeCup() {
	l := len(g.Cup)
	shaken := make([]Dice, l)
	for i, p := range rnd.Perm(l) {
		shaken[i] = g.Cup[p]
	}
	g.Cup = shaken
}

func (g *Game) TakeDice(n int) ([]Dice, []brdgme.Log) {
	if n < 0 {
		panic("Must have more than 0")
	}
	dice := []Dice{}
	logs := []brdgme.Log{}
	if n == 0 {
		return dice, logs
	}

	if len(g.Cup) < n {
		logs = append(logs, brdgme.NewPublicLog(
			"Not enough dice remaining, returning kept dice to the cup",
		))
		for _, d := range g.Kept {
			g.Cup = append(g.Cup, d.Dice)
		}
		g.Kept = DiceResultList{}
		g.ShakeCup()
	}

	dice, g.Cup = g.Cup[:n], g.Cup[n:]
	return dice, logs
}

func (g *Game) StartTurn() []brdgme.Log {
	g.Cup = AllDice()
	g.ShakeCup()
	g.Kept = DiceResultList{}
	g.CurrentRoll = DiceResultList{}
	g.RoundBrains = 0
	g.RoundShotguns = 0
	return g.Roll()
}

func (g *Game) NextPlayer() []brdgme.Log {
	logs := []brdgme.Log{}
	g.CurrentTurn = (g.CurrentTurn + 1) % g.Players
	if g.CurrentTurn == 0 {
		// Check for game end
		score, leaders := g.Leaders()
		if score >= 13 {
			if len(leaders) == 1 {
				g.Finished = true
				return logs
			}
			// Roll off!
			g.RollOffPlayers = map[int]bool{}
			parts := []string{}
			for _, l := range leaders {
				g.RollOffPlayers[l] = true
				parts = append(parts, render.Player(l))
			}
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				"It's a tied score of %s between %s, tie breaker round!",
				render.Bold(strconv.Itoa(score)),
				brdgme.CommaList(parts),
			)))
		}
	}
	if g.RollOffPlayers != nil && !g.RollOffPlayers[g.CurrentTurn] {
		logs = append(logs, g.NextPlayer()...)
	} else {
		logs = append(logs, g.StartTurn()...)
	}
	return logs
}

func (g *Game) PlayerRoll(player int) ([]brdgme.Log, error) {
	if !g.CanRoll(player) {
		return nil, errors.New("can't roll at the moment")
	}
	return g.Roll(), nil
}

func (g *Game) Roll() []brdgme.Log {
	logs := []brdgme.Log{}
	dice := g.CurrentRoll.Dice()
	diceLen := len(dice)
	if diceLen < 3 {
		takenDice, takeLogs := g.TakeDice(3 - diceLen)
		logs = append(logs, takeLogs...)
		dice = append(dice, takenDice...)
	}
	drl := RollDice(dice)
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s rolled %s",
		render.Player(g.CurrentTurn),
		drl,
	)))

	run := DiceResultList{}
	newBrains := 0
	wasShot := false
	for _, dr := range drl {
		switch dr.Face {
		case Brain:
			newBrains++
			g.Kept = append(g.Kept, dr)
		case Shotgun:
			g.RoundShotguns++
			g.Kept = append(g.Kept, dr)
			wasShot = true
		case Footprints:
			run = append(run, dr)
		}
	}
	if g.RoundShotguns >= 3 {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s got shot three times and lost %s brains!",
			render.Player(g.CurrentTurn),
			render.Bold(strconv.Itoa(g.RoundBrains)),
		)))
		logs = append(logs, g.NextPlayer()...)
		return logs
	} else if wasShot {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s has %s health remaining",
			render.Player(g.CurrentTurn),
			render.Bold(strconv.Itoa(3-g.RoundShotguns)),
		)))
	}
	g.RoundBrains += newBrains
	g.CurrentRoll = run
	return logs
}

func (g *Game) Keep(player int) ([]brdgme.Log, error) {
	if !g.CanKeep(player) {
		return nil, errors.New("can't keep at the moment")
	}
	g.Scores[g.CurrentTurn] += g.RoundBrains
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s kept %s brains, now has %s!",
		render.Player(g.CurrentTurn),
		render.Bold(strconv.Itoa(g.RoundBrains)),
		render.Bold(strconv.Itoa(g.Scores[g.CurrentTurn])),
	))}
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) IsFinished() bool {
	return g.Finished
}

func (g *Game) Leaders() (score int, players []int) {
	players = []int{0}
	for p := 0; p < g.Players; p++ {
		if g.Scores[p] > score {
			score = g.Scores[p]
			players = []int{}
		}
		if g.Scores[p] == score {
			players = append(players, p)
		}
	}
	return score, players
}

func (g *Game) WhoseTurn() []int {
	if g.IsFinished() {
		return []int{}
	}
	return []int{g.CurrentTurn}
}

func (g *Game) CanKeep(player int) bool {
	return g.CurrentTurn == player
}

func (g *Game) CanRoll(player int) bool {
	return g.CurrentTurn == player
}
