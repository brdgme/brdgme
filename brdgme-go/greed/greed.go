package greed

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

type Game struct {
	Players       int
	FirstPlayer   int
	Player        int
	Scores        map[int]int
	TurnScore     int
	RemainingDice []Die
	TakenThisRoll bool
}

var _ brdgme.Gamer = &Game{}

type Die = int

const (
	DieDollar Die = iota + 1
	DieG
	DieR
	DieE1
	DieE2
	DieD
)

var DieNames = map[Die]string{
	DieDollar: "$",
	DieG:      "G",
	DieR:      "R",
	DieE1:     "E1",
	DieE2:     "E2",
	DieD:      "D",
}

var DieFaces = []Die{
	DieDollar,
	DieG,
	DieR,
	DieE1,
	DieE2,
	DieD,
}

var DiceColours = map[Die]render.Color{
	DieDollar: render.Grey,
	DieG:      render.Yellow,
	DieR:      render.Red,
	DieE1:     render.Black,
	DieE2:     render.Green,
	DieD:      render.Cyan,
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6}
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

func (g *Game) PlayerRender(player int) string {
	return g.PubRender()
}

func (g *Game) PubRender() string {
	buf := bytes.NewBufferString("")
	cells := [][]render.Cell{
		{
			render.Cel(render.Bold("Remaining dice")),
			render.Cel(RenderDice(g.RemainingDice)),
		},
		{
			render.Cel(render.Bold("Score this turn")),
			render.Cel(strconv.Itoa(g.TurnScore)),
		},
	}
	t := render.Table(cells, 0, 1)
	buf.WriteString(t)
	buf.WriteString("\n\n")
	cells = [][]render.Cell{
		{
			render.Cel(render.Bold("Player")),
			render.Cel(render.Bold("Score")),
		},
	}
	for playerNum := 0; playerNum < g.Players; playerNum++ {
		playerName := render.Player(playerNum)
		if playerNum == g.FirstPlayer {
			playerName += " (started)"
		}
		cells = append(cells, []render.Cell{
			render.Cel(playerName),
			render.Cel(strconv.Itoa(g.Scores[playerNum])),
		})
	}
	t = render.Table(cells, 0, 1)
	buf.WriteString(t)
	return buf.String()
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 {
		return nil, errors.New("Greed requires at least two players")
	}
	g.Scores = map[int]int{}
	g.Players = players
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	g.Player = r.Int() % g.Players
	g.FirstPlayer = g.Player
	return g.StartTurn(), nil
}

func (g *Game) StartTurn() []brdgme.Log {
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"It is now %s's turn",
		render.Player(g.Player),
	))}
	g.TurnScore = 0
	g.TakenThisRoll = false
	logs = append(logs, g.Roll(6)...)
	return logs
}

func (g *Game) IsFinished() bool {
	if g.Player != g.FirstPlayer {
		return false
	}
	for _, s := range g.Scores {
		if s >= 5000 {
			return true
		}
	}
	return false
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{g.Scores[p]}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	return []int{g.Player}
}

func (g *Game) NextPlayer() []brdgme.Log {
	g.Player = (g.Player + 1) % g.Players
	if !g.IsFinished() {
		return g.StartTurn()
	}
	return nil
}

func (g *Game) Roll(n int) []brdgme.Log {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	g.RemainingDice = make([]Die, n)
	for i := 0; i < n; i++ {
		g.RemainingDice[i] = r.Int()%6 + 1
	}
	sort.IntSlice(g.RemainingDice).Sort()
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s rolled %s",
		render.Player(g.Player),
		RenderDice(g.RemainingDice),
	))}
	if len(AvailableScores(g.RemainingDice)) == 0 {
		// No dice!
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s rolled no scoring dice and lost %s points!",
			render.Player(g.Player),
			render.Bold(strconv.Itoa(g.TurnScore)),
		)))
		logs = append(logs, g.NextPlayer()...)
	}
	return logs
}

func RenderDie(value Die) string {
	return render.Markup(DieNames[value], DiceColours[value], true)
}

func RenderDice(values []Die) string {
	strs := make([]string, len(values))
	for i, v := range values {
		strs[i] = RenderDie(v)
	}
	return strings.Join(strs, " ")
}
