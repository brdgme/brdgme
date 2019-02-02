package age_of_war

import (
	"errors"
	"fmt"
	"math/rand"
	"time"

	"github.com/brdgme-go/brdgme"
	"github.com/brdgme-go/render"
)

var rnd = rand.New(rand.NewSource(time.Now().UnixNano()))

type Game struct {
	CurrentPlayer int
	Players       int

	Conquered    map[int]bool
	CastleOwners map[int]int

	CurrentlyAttacking int
	CompletedLines     map[int]bool
	CurrentRoll        []int
}

var _ brdgme.Gamer = &Game{}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6}
}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		return brdgme.StatusFinished{
			Placings: g.Placings(),
			Stats:    []interface{}{},
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn:  g.WhoseTurn(),
		Eliminated: []int{},
	}.ToStatus()
}

func (g *Game) Command(player int, input string, playerNames []string) (brdgme.CommandResponse, error) {
	parseOutput, err := g.CommandParser(player).Parse(input, playerNames)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	switch value := parseOutput.Value.(type) {
	case attackCommand:
		return g.AttackCommand(player, value.castle, parseOutput.Remaining)
	case lineCommand:
		return g.LineCommand(player, value.line, parseOutput.Remaining)
	case rollCommand:
		return g.RollCommand(player, parseOutput.Remaining)
	}
	return brdgme.CommandResponse{}, errors.New("inexhaustive command handler")
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 6 {
		return nil, errors.New("only for 2 to 6 players")
	}
	g.Players = players

	g.Conquered = map[int]bool{}
	g.CastleOwners = map[int]int{}
	g.CompletedLines = map[int]bool{}

	return []brdgme.Log{g.StartTurn()}, nil
}

func (g *Game) PubState() interface{} {
	return g
}

func (g *Game) PlayerState(player int) interface{} {
	return g.PubState()
}

func (g *Game) StartTurn() brdgme.Log {
	g.CurrentlyAttacking = -1
	g.CompletedLines = map[int]bool{}
	return g.Roll(7)
}

func (g *Game) NextTurn() brdgme.Log {
	g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players
	return g.StartTurn()
}

func (g *Game) CheckEndOfTurn() (bool, []brdgme.Log) {
	logs := []brdgme.Log{}
	if g.CurrentlyAttacking != -1 {
		c := Castles[g.CurrentlyAttacking]
		lines := c.CalcLines(
			g.Conquered[g.CurrentlyAttacking],
		)
		// If the player has completed all lines, they take the card and it is
		// the end of the turn.
		allLines := true
		for l := range lines {
			if !g.CompletedLines[l] {
				allLines = false
				break
			}
		}
		if allLines {
			suffix := ""
			if g.Conquered[g.CurrentlyAttacking] {
				suffix = fmt.Sprintf(
					" from %s",
					render.Player(g.CastleOwners[g.CurrentlyAttacking]),
				)
			}
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				"%s conquered the castle %s%s",
				render.Player(g.CurrentPlayer),
				c.RenderName(),
				suffix,
			)))
			g.Conquered[g.CurrentlyAttacking] = true
			g.CastleOwners[g.CurrentlyAttacking] = g.CurrentPlayer
			if clanConquered, _ := g.ClanConquered(c.Clan); clanConquered {
				logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
					"%s conquered the clan %s",
					render.Player(g.CurrentPlayer),
					RenderClan(c.Clan),
				)))
			}
			logs = append(logs, g.NextTurn())
			return true, logs
		}

		// If the player doesn't have enough dice to complete the rest of the
		// lines, it is the end of the turn.
		reqDice := 0
		numDice := len(g.CurrentRoll)
		canAffordLine := false
		for i, l := range lines {
			if g.CompletedLines[i] {
				continue
			}
			reqDice += l.MinDice()
			if reqDice > numDice {
				logs = append(logs, g.FailedAttackMessage(), g.NextTurn())
				return false, logs
			}
			if can, _ := l.CanAfford(g.CurrentRoll); can {
				canAffordLine = true
			}
		}

		// If the player has the minimum required dice but they can't afford a
		// line, it is the end of the turn.
		if reqDice == numDice && !canAffordLine {
			logs = append(logs, g.FailedAttackMessage(), g.NextTurn())
			return false, logs
		}
	} else {
		// If the player doesn't have enough dice for any castle, it is the end
		// of the turn.
		for i, c := range Castles {
			if g.Conquered[i] && g.CastleOwners[i] == g.CurrentPlayer {
				// They already own it.
				continue
			}
			if conquered, _ := g.ClanConquered(c.Clan); conquered {
				// The clan is conquered, can't steal.
				continue
			}
			minDice := c.MinDice()
			if g.Conquered[i] {
				minDice++
			}
			if minDice <= len(g.CurrentRoll) {
				// They can afford this one
				return false, logs
			}
		}
		// They couldn't afford anything, next turn.
		logs = append(logs, g.FailedAttackMessage(), g.NextTurn())
		return false, logs
	}
	return false, logs
}

func (g *Game) FailedAttackMessage() brdgme.Log {
	target := "anything"
	if g.CurrentlyAttacking != -1 {
		target = Castles[g.CurrentlyAttacking].RenderName()
	}
	return brdgme.NewPublicLog(fmt.Sprintf(
		"%s failed to conquer %s",
		render.Player(g.CurrentPlayer),
		target,
	))
}

func (g *Game) Scores() map[int]int {
	scores := map[int]int{}
	conqueredClans := map[int]bool{}
	for cIndex, c := range Castles {
		if !g.Conquered[cIndex] {
			continue
		}
		clanConquered, ok := conqueredClans[c.Clan]
		if !ok {
			var conqueredBy int
			clanConquered, conqueredBy = g.ClanConquered(c.Clan)
			conqueredClans[c.Clan] = clanConquered
			if clanConquered {
				scores[conqueredBy] += ClanSetPoints[c.Clan]
			}
		}
		if clanConquered {
			continue
		}
		scores[g.CastleOwners[cIndex]] += c.Points
	}
	return scores
}

func (g *Game) Points() []float32 {
	scores := g.Scores()
	points := make([]float32, g.Players)
	for p := 0; p < g.Players; p++ {
		points[p] = float32(scores[p])
	}
	return points
}

func (g *Game) IsFinished() bool {
	return len(g.Conquered) == len(Castles)
}

func (g *Game) Placings() []int {
	// Winner is determined by score, with ties broken by conquered clans.
	playerConqueredClans := map[int]int{}
	for _, clan := range Clans {
		if conquered, by := g.ClanConquered(clan); conquered {
			playerConqueredClans[by]++
		}
	}
	scores := g.Scores()
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{scores[p], playerConqueredClans[p]}
	}

	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) ClanConquered(clan int) (conquered bool, player int) {
	player = -1
	conquered = true
	for i, c := range Castles {
		if c.Clan != clan {
			continue
		}
		if !g.Conquered[i] {
			conquered = false
			return
		}
		if player == -1 {
			player = g.CastleOwners[i]
		} else if player != g.CastleOwners[i] {
			conquered = false
			return
		}
	}
	return
}
