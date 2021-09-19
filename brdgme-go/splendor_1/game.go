package splendor_1

import (
	"errors"
	"regexp"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcost"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	MaxGold   = 5
	MaxTokens = 10
)

type Phase int

const (
	PhaseMain Phase = iota
	PhaseVisit
	PhaseDiscard
)

type Game struct {
	Players int

	Decks  [3][]Card
	Board  [3][]Card
	Nobles []Noble
	Tokens libcost.Cost

	PlayerBoards []PlayerBoard

	CurrentPlayer int
	Phase         Phase

	EndTriggered bool
	Ended        bool
}

var _ brdgme.Gamer = &Game{}

var LocRegexp = regexp.MustCompile(`^([\dA-Z])([\dA-Z])$`)

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 4 {
		return nil, errors.New("must be between 2 and 4 players")
	}

	g.Players = players

	g.Decks = [3][]Card{}
	g.Board = [3][]Card{}
	for l, cards := range [3][]Card{
		ShuffleCards(Level1Cards()),
		ShuffleCards(Level2Cards()),
		ShuffleCards(Level3Cards()),
	} {
		g.Board[l] = cards[:4]
		g.Decks[l] = cards[4:]
	}

	g.Nobles = ShuffleNobles(NobleCards())[:players+1]

	g.Tokens = libcost.Cost{
		Gold: MaxGold,
	}
	maxGems := g.MaxGems()
	for _, r := range Gems {
		g.Tokens[r] = maxGems
	}

	g.PlayerBoards = make([]PlayerBoard, g.Players)
	for p := 0; p < players; p++ {
		g.PlayerBoards[p] = NewPlayerBoard()
	}

	return nil, nil
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4}
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
		points[p] = float32(g.PlayerBoards[p].Prestige())
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

func (g *Game) MaxGems() int {
	switch g.Players {
	case 2:
		return 4
	case 3:
		return 5
	default:
		return 7
	}
}

func (g *Game) IsFinished() bool {
	return g.Ended
}

func (g *Game) CheckEndTriggered() []brdgme.Log {
	if g.EndTriggered {
		return nil
	}
	for p := 0; p < g.Players; p++ {
		if g.PlayerBoards[p].Prestige() >= 15 {
			g.EndTriggered = true
			return []brdgme.Log{brdgme.NewPublicLog(render.Bold(
				"The end of the game has been triggered",
			))}
		}
	}
	return nil
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{
			g.PlayerBoards[p].Prestige(),
			len(g.PlayerBoards[p].Cards),
		}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) NextPhase() []brdgme.Log {
	switch g.Phase {
	case PhaseMain:
		return g.VisitPhase()
	case PhaseVisit:
		return g.DiscardPhase()
	case PhaseDiscard:
		return g.NextPlayer()
	}
	panic("invalid phase")
}

func (g *Game) VisitPhase() []brdgme.Log {
	g.Phase = PhaseVisit
	pb := g.PlayerBoards[g.CurrentPlayer]
	canVisit := []int{}
	for i, n := range g.Nobles {
		if CanAfford(pb.Bonuses(), n.Cost) {
			canVisit = append(canVisit, i)
		}
	}
	switch len(canVisit) {
	case 0:
		return g.NextPhase()
	case 1:
		logs, err := g.Visit(g.CurrentPlayer, canVisit[0])
		if err != nil {
			// invariant
			panic(err)
		}
		return logs
	}
	return nil
}

func (g *Game) DiscardPhase() []brdgme.Log {
	g.Phase = PhaseDiscard
	if g.PlayerBoards[g.CurrentPlayer].Tokens.Sum() <= MaxTokens {
		return g.NextPhase()
	}
	return nil
}

func (g *Game) NextPlayer() []brdgme.Log {
	logs := g.CheckEndTriggered()
	g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players
	if g.EndTriggered && g.CurrentPlayer == 0 {
		g.Ended = true
	} else {
		g.MainPhase()
	}
	return logs
}

func (g *Game) MainPhase() {
	g.Phase = PhaseMain
}

func (g *Game) Pay(player int, amount libcost.Cost) error {
	if !g.PlayerBoards[player].CanAfford(amount) {
		return errors.New("can't afford that")
	}
	offset := g.PlayerBoards[player].Bonuses().Sub(amount)
	for _, gem := range Gems {
		if offset[gem] < 0 {
			// Player didn't have enough just with bonuses
			g.PlayerBoards[player].Tokens[gem] += offset[gem]
			g.Tokens[gem] -= offset[gem]
			if g.PlayerBoards[player].Tokens[gem] < 0 {
				// Player didn't have enough normal tokens either, use gold
				g.PlayerBoards[player].Tokens[Gold] +=
					g.PlayerBoards[player].Tokens[gem]
				g.Tokens[gem] += g.PlayerBoards[player].Tokens[gem]
				g.Tokens[Gold] -= g.PlayerBoards[player].Tokens[gem]
				g.PlayerBoards[player].Tokens[gem] = 0
			}
		}
	}
	return nil
}
