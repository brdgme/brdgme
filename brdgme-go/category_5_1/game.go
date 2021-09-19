package category_5_1

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"strconv"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const EndScore = 66

var r = rand.New(rand.NewSource(time.Now().UnixNano()))

type Game struct {
	Players      int
	Deck         []Card
	Discard      []Card
	PlayerPoints map[int]int
	Hands        map[int][]Card
	PlayerCards  map[int][]Card
	Plays        map[int]Card
	Board        [4][]Card
	Resolving    bool
	ChoosePlayer int
}

var _ brdgme.Gamer = &Game{}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 10 {
		return nil, errors.New("this game is for 2-10 players")
	}
	g.Players = players
	g.Deck = Shuffle(Deck())
	g.Discard = []Card{}
	g.PlayerPoints = map[int]int{}
	g.Hands = map[int][]Card{}
	g.PlayerCards = map[int][]Card{}
	g.Plays = map[int]Card{}
	g.Board = [4][]Card{{}, {}, {}, {}}
	for p := 0; p < g.Players; p++ {
		g.Hands[p] = []Card{}
		g.PlayerCards[p] = []Card{}
	}
	return g.StartRound(), nil
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6, 7, 8, 9, 10}
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

func (g *Game) StartRound() []brdgme.Log {
	// Discard cards on the board
	for i, b := range g.Board {
		g.DiscardCards(b)
		g.Board[i] = []Card{}
	}
	// Discard the player cards
	for p := 0; p < g.Players; p++ {
		g.DiscardCards(g.PlayerCards[p])
		g.PlayerCards[p] = []Card{}
	}
	// Start each row with a card
	for i := range g.Board {
		g.Board[i] = append(g.Board[i], g.DrawCards(1)...)
	}
	// Each player gets 10 cards
	for p := 0; p < g.Players; p++ {
		g.Hands[p] = SortCards(g.DrawCards(10))
	}
	return []brdgme.Log{brdgme.NewPublicLog(
		"Starting a new round, dealing 10 cards to each player",
	)}
}

func (g *Game) ResolvePlays() []brdgme.Log {
	g.Resolving = true
	logs := []brdgme.Log{}
	for {
		// Find who has the next lowest card
		lowestCard := Card(0)
		lowestPlayer := 0
		for p := 0; p < g.Players; p++ {
			if g.Plays[p] == 0 {
				continue
			}
			if lowestCard == 0 || g.Plays[p] < lowestCard {
				lowestCard = g.Plays[p]
				lowestPlayer = p
			}
		}
		if lowestCard == 0 {
			// None left, we've resolved all
			break
		}
		// Find which row it goes in
		closestCard := Card(0)
		closestRow := 0
		for i, row := range g.Board {
			lastCard := row[len(row)-1]
			if lastCard < lowestCard && (closestCard == 0 || lastCard > closestCard) {
				closestCard = lastCard
				closestRow = i
			}
		}
		if closestCard == 0 {
			// The card is lower than all rows, player gets to choose row
			g.ChoosePlayer = lowestPlayer
			return logs
		} else if len(g.Board[closestRow]) == 5 {
			// Row is full, gotta take it
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				"%s played %s as card %s of row %s and {{b}}took the row for %d points{{/b}}",
				render.Player(lowestPlayer),
				lowestCard,
				render.Bold(strconv.Itoa(len(g.Board[closestRow])+1)),
				render.Bold(strconv.Itoa(closestRow+1)),
				CardsHeads(g.Board[closestRow]),
			)))
			g.PlayerCards[lowestPlayer] = append(
				g.PlayerCards[lowestPlayer], g.Board[closestRow]...)
			g.Board[closestRow] = []Card{lowestCard}
		} else {
			// Just slot the card into the row
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				"%s played %s as card %s of row %s",
				render.Player(lowestPlayer),
				lowestCard,
				render.Bold(strconv.Itoa(len(g.Board[closestRow])+1)),
				render.Bold(strconv.Itoa(closestRow+1)),
			)))
			g.Board[closestRow] = append(g.Board[closestRow], lowestCard)
		}
		g.Plays[lowestPlayer] = 0
	}
	g.Resolving = false
	switch len(g.Hands[0]) {
	case 0:
		logs = append(logs, g.EndRound()...)
	case 1:
		// Automatically play last card
		for p := 0; p < g.Players; p++ {
			playLogs, err := g.Play(p, g.Hands[p][0])
			if err != nil {
				// Game should only play valid cards
				panic(err)
			}
			logs = append(logs, playLogs...)
		}
	}
	return logs
}

func (g *Game) EndRound() []brdgme.Log {
	buf := bytes.NewBufferString(render.Bold("End of the round, counting points"))
	for p := 0; p < g.Players; p++ {
		total := 0
		for _, c := range g.PlayerCards[p] {
			total += c.Heads()
		}
		g.PlayerPoints[p] += total
		buf.WriteString(fmt.Sprintf(
			"\n  %s had %s cards worth %s points, total now %s",
			render.Player(p),
			render.Bold(strconv.Itoa(len(g.PlayerCards[p]))),
			render.Bold(strconv.Itoa(total)),
			render.Bold(strconv.Itoa(g.PlayerPoints[p])),
		))
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(buf.String())}
	if !g.IsFinished() {
		logs = append(logs, g.StartRound()...)
	}
	return logs
}

func (g *Game) DrawCards(n int) []Card {
	cards := []Card{}
	if l := len(g.Deck); l >= n {
		cards, g.Deck = TakeCards(g.Deck, n)
	} else {
		cards = append(cards, g.Deck...)
		g.Deck = Shuffle(g.Discard)
		g.Discard = []Card{}
		cards = append(cards, g.DrawCards(n-l)...)
	}
	return cards
}

func (g *Game) DiscardCards(cards []Card) {
	g.Discard = append(g.Discard, cards...)
}

func (g *Game) IsFinished() bool {
	highestScore := 0
	for p := 0; p < g.Players; p++ {
		if g.PlayerPoints[p] > highestScore {
			highestScore = g.PlayerPoints[p]
		}
		if len(g.Hands[p]) > 0 {
			return false
		}
	}
	return highestScore >= EndScore
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{-g.PlayerPoints[p]}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	if g.Resolving {
		return []int{g.ChoosePlayer}
	}
	whose := []int{}
	for p := 0; p < g.Players; p++ {
		if g.Plays[p] == 0 {
			whose = append(whose, p)
		}
	}
	return whose
}
