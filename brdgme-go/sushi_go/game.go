package sushi_go

import (
	"errors"
	"fmt"
	"math/rand"
	"strconv"
	"strings"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const Dummy = 2

var rnd = rand.New(rand.NewSource(time.Now().UnixNano()))

type Game struct {
	Players    int
	AllPlayers int

	Round int

	Deck    []int
	Hands   [][]int
	Playing map[int][]int

	Played       map[int][]int
	PlayerPoints map[int]int

	Controller int // For 2 players, who is controlling the dummy this turn
}

var _ brdgme.Gamer = &Game{}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5}
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
		points[p] = float32(g.PlayerPoints[p])
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

func (g *Game) New(players int) ([]brdgme.Log, error) {
	_, ok := PlayerDrawCounts[players]
	if !ok {
		return nil, errors.New("requires between 2 and 5 players")
	}

	logs := []brdgme.Log{}
	g.Players = players
	g.AllPlayers = players
	if players == 2 {
		g.AllPlayers = g.Players + 1
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Because there are only two players, you will be joined by %s",
			g.RenderName(Dummy),
		)))
	}

	g.Deck = Shuffle(Deck())
	g.Playing = map[int][]int{}
	g.Played = map[int][]int{}
	g.PlayerPoints = map[int]int{}
	logs = append(logs, g.StartRound()...)
	return logs, nil
}

func (g *Game) StartRound() []brdgme.Log {
	logs := []brdgme.Log{}
	g.Round++
	for p := 0; p < g.AllPlayers; p++ {
		// Remove anything that's not a pudding
		newPlayed := []int{}
		for _, c := range g.Played[p] {
			if c == CardPudding {
				newPlayed = append(newPlayed, c)
			}
		}
		g.Played[p] = newPlayed
	}
	g.Hands = make([][]int, g.AllPlayers)
	drawCount := PlayerDrawCounts[g.AllPlayers]
	passDir := "left"
	if g.Round == 2 {
		passDir = "right"
	}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"Starting round %s, hands will be passed to the %s.  Dealing %s cards to each player",
		render.Bold(strconv.Itoa(g.Round)),
		render.Bold(passDir),
		render.Bold(strconv.Itoa(drawCount)),
	)))
	for p := 0; p < g.AllPlayers; p++ {
		g.Hands[p], g.Deck = g.Deck[:drawCount], g.Deck[drawCount:]
		g.Hands[p] = Sort(g.Hands[p])
	}
	logs = append(logs, g.StartHand()...)
	return logs
}

func (g *Game) StartHand() []brdgme.Log {
	logs := []brdgme.Log{}
	if g.Players == 2 {
		// Controller draws a card from the dummy hand.
		i := rnd.Int() % len(g.Hands[Dummy])
		logs = append(logs, brdgme.NewPrivateLog(fmt.Sprintf(
			"You drew %s from %s",
			RenderCard(g.Hands[Dummy][i]),
			g.RenderName(Dummy),
		), []int{g.Controller}))
		g.Hands[g.Controller] = append(g.Hands[g.Controller], g.Hands[Dummy][i])
		g.Hands[g.Controller] = Sort(g.Hands[g.Controller])
		g.Hands[Dummy] = append(g.Hands[Dummy][:i], g.Hands[Dummy][i+1:]...)
	}
	return logs
}

func (g *Game) EndHand() []brdgme.Log {
	logs := []brdgme.Log{}
	// Play cards
	for p := 0; p < g.AllPlayers; p++ {
		g.Hands[p] = TrimPlayed(g.Hands[p])
		g.Played[p] = append(g.Played[p], g.Playing[p]...)
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s played %s",
			g.RenderName(p),
			brdgme.CommaList(RenderCards(g.Playing[p])),
		)))
		if len(g.Playing[p]) == 2 {
			// Use chopsticks.
			if i, ok := Contains(CardChopsticks, g.Played[p]); ok {
				g.Hands[p] = append(g.Hands[p], CardChopsticks)
				g.Played[p] = append(g.Played[p][:i], g.Played[p][i+1:]...)
			}
		}
		g.Playing[p] = nil
	}
	if g.Players == 2 {
		// Next player controls the dummy
		g.Controller = (g.Controller + 1) % g.Players
	}
	// End round if we're out of cards
	if len(g.Hands[0]) == 0 {
		logs = append(logs, g.EndRound()...)
		return logs
	}
	// Pass hands
	if g.Players == 2 {
		logs = append(logs, brdgme.NewPublicLog("Players are swapping hands"))
		g.Hands[0], g.Hands[1] = g.Hands[1], g.Hands[0]
	} else if g.Round%2 == 1 {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Passing hands to the %s",
			render.Bold("left"),
		)))
		extra := g.Hands[0]
		g.Hands = append(g.Hands[1:], extra)
	} else {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Passing hands to the %s",
			render.Bold("right"),
		)))
		l := len(g.Hands)
		extra := g.Hands[l-1]
		g.Hands = append([][]int{extra}, g.Hands[:l-1]...)
	}
	logs = append(logs, g.StartHand()...)
	return logs
}

func (g *Game) Score() ([]int, []string) {
	scores := make([]int, g.AllPlayers)
	output := []string{}

	// Score maki
	maki := map[int]int{}
	for p := 0; p < g.AllPlayers; p++ {
		for _, c := range g.Played[p] {
			switch c {
			case CardMakiRoll1:
				maki[p]++
			case CardMakiRoll2:
				maki[p] += 2
			case CardMakiRoll3:
				maki[p] += 3
			}
		}
	}
	first := 0
	firstPlayers := []int{}
	second := 0
	secondPlayers := []int{}
	for p, m := range maki {
		if m > first {
			second = first
			secondPlayers = firstPlayers
			first = m
			firstPlayers = []int{}
		}
		if m == first {
			firstPlayers = append(firstPlayers, p)
		} else {
			if m > second {
				second = m
				secondPlayers = []int{}
			}
			if m == second {
				secondPlayers = append(secondPlayers, p)
			}
		}
	}
	makiRollsStr := render.Markup("maki rolls", CardColours[CardMakiRoll1], true)
	if first == 0 {
		output = append(output, fmt.Sprintf(
			"Nobody had %s, no points awarded",
			makiRollsStr,
		))
	} else {
		firstPoints := 6 / len(firstPlayers)
		output = append(output, fmt.Sprintf(
			"%s had %s %s, awarding %s points",
			brdgme.CommaList(g.RenderNames(firstPlayers)),
			render.Bold(strconv.Itoa(first)),
			makiRollsStr,
			render.Bold(strconv.Itoa(firstPoints)),
		))
		for _, p := range firstPlayers {
			scores[p] += firstPoints
		}
		if len(firstPlayers) == 1 && second > 0 && len(secondPlayers) <= 3 {
			secondPoints := 3 / len(secondPlayers)
			output = append(output, fmt.Sprintf(
				"%s had %s %s, awarding %s points",
				brdgme.CommaList(g.RenderNames(secondPlayers)),
				render.Bold(strconv.Itoa(second)),
				makiRollsStr,
				render.Bold(strconv.Itoa(secondPoints)),
			))
			for _, p := range secondPlayers {
				scores[p] += secondPoints
			}
		}
	}

	if g.Round == 3 {
		// Score puddings
		pudding := map[int]int{}
		for p := 0; p < g.AllPlayers; p++ {
			for _, c := range g.Played[p] {
				if c == CardPudding {
					pudding[p]++
				}
			}
		}
		first := 0
		firstPlayers := []int{}
		last := 0
		lastPlayers := []int{}
		for p := 0; p < g.AllPlayers; p++ {
			c := pudding[p]
			if c > first {
				first = c
				firstPlayers = []int{}
			}
			if c == first {
				firstPlayers = append(firstPlayers, p)
			}
			if c < last || len(lastPlayers) == 0 {
				last = c
				lastPlayers = []int{}
			}
			if c == last {
				lastPlayers = append(lastPlayers, p)
			}
		}
		puddingsStr := render.Markup("puddings", CardColours[CardPudding], true)
		if first == last {
			output = append(output, fmt.Sprintf(
				"Everybody had the same number of %s, no points awarded",
				puddingsStr,
			))
		} else {
			firstPoints := 6 / len(firstPlayers)
			output = append(output, fmt.Sprintf(
				"%s had %s %s, awarding %s points",
				brdgme.CommaList(g.RenderNames(firstPlayers)),
				render.Bold(strconv.Itoa(first)),
				puddingsStr,
				render.Bold(strconv.Itoa(firstPoints)),
			))
			for _, p := range firstPlayers {
				scores[p] += firstPoints
			}
			if g.Players != 2 {
				lastPoints := -6 / len(lastPlayers)
				output = append(output, fmt.Sprintf(
					"%s had %s %s, awarding %s points",
					brdgme.CommaList(g.RenderNames(lastPlayers)),
					render.Bold(strconv.Itoa(last)),
					puddingsStr,
					render.Bold(strconv.Itoa(lastPoints)),
				))
				for _, p := range lastPlayers {
					scores[p] += lastPoints
				}
			}
		}
	}

	// Score normal cards
	for p := 0; p < g.AllPlayers; p++ {
		output = append(output, fmt.Sprintf(
			render.Bold("Scoring cards for %s"),
			g.RenderName(p),
		))
		cardCounts := map[int]int{}
		for _, c := range g.Played[p] {
			if s, ok := CardBaseScores[c]; ok {
				text := RenderCard(c)
				if cardCounts[CardWasabi] > 0 {
					s *= 3
					cardCounts[CardWasabi]--
					text = fmt.Sprintf("%s + %s", text, RenderCard(CardWasabi))
				}
				output = append(output, fmt.Sprintf(
					"%s, %s points",
					text,
					render.Bold(strconv.Itoa(s)),
				))
				scores[p] += s
			} else {
				cardCounts[c]++
			}
		}
		if s := cardCounts[CardTempura] / 2 * 5; s > 0 {
			output = append(output, fmt.Sprintf(
				"%d x %s, %s points",
				cardCounts[CardTempura],
				RenderCard(CardTempura),
				render.Bold(strconv.Itoa(s)),
			))
			scores[p] += s
		}
		if s := cardCounts[CardSashimi] / 3 * 10; s > 0 {
			output = append(output, fmt.Sprintf(
				"%d x %s, %s points",
				cardCounts[CardSashimi],
				RenderCard(CardSashimi),
				render.Bold(strconv.Itoa(s)),
			))
			scores[p] += s
		}
		if n := cardCounts[CardDumpling]; n > 0 {
			s := (n*n + n) / 2
			if s > 15 {
				s = 15
			}
			output = append(output, fmt.Sprintf(
				"%d x %s, %s points",
				cardCounts[CardDumpling],
				RenderCard(CardDumpling),
				render.Bold(strconv.Itoa(s)),
			))
			scores[p] += s
		}
	}
	return scores, output
}

func (g *Game) EndRound() []brdgme.Log {
	logs := []brdgme.Log{}
	scores, output := g.Score()
	output = append(output, render.Bold("The scores after this round are:"))
	for p := 0; p < g.AllPlayers; p++ {
		g.PlayerPoints[p] += scores[p]
		output = append(output, fmt.Sprintf(
			"%s: %s points",
			g.RenderName(p),
			render.Bold(strconv.Itoa(g.PlayerPoints[p])),
		))
	}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s\n%s",
		render.Bold(fmt.Sprintf(
			"It is the end of round %d, scoring\n",
			g.Round,
		)),
		strings.Join(output, "\n"),
	)))
	if g.Round < 3 {
		logs = append(logs, g.StartRound()...)
	}
	return logs
}

func (g *Game) IsFinished() bool {
	return g.Round == 3 && len(g.Hands[0]) == 0 && g.Playing[0] == nil
}

func (g *Game) PuddingCards(player int) int {
	numPudding := 0
	for _, c := range g.Played[player] {
		if c == CardPudding {
			numPudding++
		}
	}
	return numPudding
}

func (g *Game) Placings() []int {
	metrics := [][]int{}
	for p := 0; p < g.AllPlayers; p++ {
		// Number of points, ties broken by number of pudding cards
		metrics[p] = []int{g.PlayerPoints[p], g.PuddingCards(p)}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	if g.IsFinished() {
		return []int{}
	}
	whose := []int{}
	for pNum := 0; pNum < g.Players; pNum++ {
		if g.CanPlay(pNum) || g.CanDummy(pNum) {
			whose = append(whose, pNum)
		}
	}
	return whose
}

func (g *Game) RenderName(player int) string {
	if player > g.Players-1 {
		// It's the dummy
		return render.Markup("<dummy>", render.Grey, true)
	}
	return render.Player(player)
}

func (g *Game) RenderNames(players []int) []string {
	playerStrs := make([]string, len(players))
	for i, p := range players {
		playerStrs[i] = g.RenderName(p)
	}
	return playerStrs
}
