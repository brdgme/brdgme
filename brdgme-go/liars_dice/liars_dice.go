package liars_dice

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"strings"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	die "github.com/brdgme/brdgme/brdgme-go/libdie"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	START_DICE_COUNT = 5
)

var (
	NormalDiceColour = render.Black
	WildDiceColour   = render.Cyan
)

type Game struct {
	Players       int
	CurrentPlayer int
	PlayerDice    [][]int
	BidQuantity   int
	BidValue      int
	BidPlayer     int
}

type PubState struct {
	Players       int
	CurrentPlayer int
	BidQuantity   int
	BidValue      int
	BidPlayer     int
}

type PlayerState struct {
	Dice []int
	Game PubState
}

var _ brdgme.Gamer = &Game{}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		return brdgme.StatusFinished{
			Placings: g.Placings(),
			Stats:    []interface{}{},
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn:  g.WhoseTurn(),
		Eliminated: g.EliminatedPlayerList(),
	}.ToStatus()
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{len(g.PlayerDice[p])}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6}
}

func (g *Game) PlayerState(player int) interface{} {
	dice := []int{}
	if player > 0 && len(g.PlayerDice) < player {
		dice = g.PlayerDice[player]
	}
	return PlayerState{
		Dice: dice,
		Game: g.PubState().(PubState),
	}
}

func (g *Game) PubState() interface{} {
	return PubState{
		Players:       g.Players,
		CurrentPlayer: g.CurrentPlayer,
		BidQuantity:   g.BidQuantity,
		BidValue:      g.BidValue,
		BidPlayer:     g.BidPlayer,
	}
}

func (g *Game) Points() []float32 {
	points := make([]float32, len(g.PlayerDice))
	for p, d := range g.PlayerDice {
		points[p] = float32(len(d))
	}
	return points
}

func (g *Game) PlayerRender(player int) string {
	return g.Render(&player)
}

func (g *Game) PubRender() string {
	return g.Render(nil)
}

func (g *Game) Render(player *int) string {
	buf := bytes.NewBufferString("")
	currentBidText := render.Fg(render.Grey, "first bid")
	if g.BidQuantity != 0 {
		currentBidText = RenderBid(g.BidQuantity, g.BidValue)
	}
	buf.WriteString(fmt.Sprintf("Current bid: %s\n", currentBidText))
	if player != nil && len(g.PlayerDice[*player]) > 0 {
		buf.WriteString(fmt.Sprintf("Your dice: %s\n\n",
			render.Bold(strings.Join(RenderDice(g.PlayerDice[*player]), " "))))
	}
	cells := [][]render.Cell{
		[]render.Cell{
			render.Cel(render.Bold("Player"), render.Left),
			render.Cel(render.Bold("Remaining dice"), render.Left),
		},
	}
	for pNum := 0; pNum < g.Players; pNum++ {
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(pNum), render.Left),
			render.Cel(fmt.Sprintf("%d", len(g.PlayerDice[pNum])), render.Left),
		})
	}
	table := render.Table(cells, 0, 1)
	buf.WriteString(table)
	return buf.String()
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	// Set players
	if players < 2 || players > 6 {
		return nil, errors.New("Liar's Dice must be between 2 and 6 players")
	}
	g.Players = players
	// Set a random first player
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	g.CurrentPlayer = r.Int() % g.Players
	// Initialise dice
	g.PlayerDice = make([][]int, g.Players)
	for pNum := 0; pNum < g.Players; pNum++ {
		g.PlayerDice[pNum] = make([]int, START_DICE_COUNT)
	}
	// Kick off the first round
	g.StartRound()
	return nil, nil
}

func (g *Game) StartRound() {
	g.BidQuantity = 0
	g.RollDice()
}

func (g *Game) RollDice() {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	for pNum := 0; pNum < g.Players; pNum++ {
		for d, _ := range g.PlayerDice[pNum] {
			g.PlayerDice[pNum][d] = (r.Int() % 6) + 1
		}
	}
}

func (g *Game) IsFinished() bool {
	return len(g.ActivePlayers()) < 2
}

func (g *Game) Winners() []int {
	if g.IsFinished() {
		return []int{g.ActivePlayers()[0]}
	}
	return []int{}
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) ActivePlayers() []int {
	players := []int{}
	for pNum := 0; pNum < g.Players; pNum++ {
		if len(g.PlayerDice[pNum]) > 0 {
			players = append(players, pNum)
		}
	}
	return players
}

func (g *Game) NextActivePlayer(from int) int {
	next := (from + 1) % g.Players
	for len(g.PlayerDice[next]) == 0 && next != from {
		next = (next + 1) % g.Players
	}
	return next
}

func (g *Game) EliminatedPlayerList() []int {
	eliminated := []int{}
	for pNum := 0; pNum < g.Players; pNum++ {
		if len(g.PlayerDice[pNum]) == 0 {
			eliminated = append(eliminated, pNum)
		}
	}
	return eliminated
}

func RenderBid(quantity int, value int) string {
	suffix := ""
	if quantity > 1 {
		suffix = "s"
	}
	return fmt.Sprintf(
		"%s %s%s",
		brdgme.NumberStr(quantity),
		RenderDie(value),
		suffix,
	)
}

func RenderDie(value int) string {
	c := NormalDiceColour
	if value == 1 {
		c = WildDiceColour
	}
	return render.Bold(render.Fg(c, die.Render(value)))
}

func RenderDice(values []int) []string {
	strs := make([]string, len(values))
	for i, v := range values {
		strs[i] = RenderDie(v)
	}
	return strs
}
