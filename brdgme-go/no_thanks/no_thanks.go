package no_thanks

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
	Players         int
	PlayerHands     [][]int
	PlayerChips     []int
	CentreChips     int
	RemainingCards  []int
	CurrentlyMoving int
}

var _ brdgme.Gamer = &Game{}

var RenderNoCards = render.Fg(render.Grey, "no cards")

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{3, 4, 5}
}

func (g *Game) PlayerState(player int) interface{} {
	return nil
}

func (g *Game) PubState() interface{} {
	return nil
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
	buf := bytes.NewBufferString("")
	if !g.IsFinished() {
		buf.WriteString(render.Bold(fmt.Sprintf(
			"Current card:  %s",
			RenderCard(g.PeekTopCard()),
		)))
		buf.WriteString(fmt.Sprintf(
			" (%d cards remaining)\n",
			len(g.RemainingCards)-1,
		))

		buf.WriteString(render.Bold(fmt.Sprintf(
			"Current chips: %s\n\n",
			RenderChips(g.CentreChips),
		)))

		if player >= 0 {
			buf.WriteString(render.Bold("Your hand:  "))
			if len(g.PlayerHands[player]) > 0 {
				buf.WriteString(g.RenderCardsForPlayer(player, g.PeekTopCard()))
			} else {
				buf.WriteString(RenderNoCards)
			}
			buf.WriteString("\n")
			buf.WriteString(render.Bold("Your chips: "))
			buf.WriteString(RenderChips(g.PlayerChips[player]))
			buf.WriteString("\n\n")
		}
	}
	header := []render.Cell{
		render.Cel(render.Bold("Players")),
		render.Cel(render.Bold("Cards")),
	}
	if g.IsFinished() {
		header = append(header, render.Cel(render.Bold("Score")))
	}
	cells := [][]render.Cell{
		header,
	}
	for pNum := 0; pNum < g.Players; pNum++ {
		row := []render.Cell{
			render.Cel(render.Player(pNum)),
		}
		if len(g.PlayerHands[pNum]) > 0 {
			row = append(row,
				render.Cel(g.RenderCardsForPlayer(pNum, g.PeekTopCard())))
		} else {
			row = append(row, render.Cel(RenderNoCards))
		}
		if g.IsFinished() {
			row = append(row, render.Cel(fmt.Sprintf(
				"%s chips, %s points",
				render.Bold(RenderChips(g.PlayerChips[pNum])),
				render.Bold(RenderPoints(g.FinalPlayerScore(pNum))),
			)))
		}
		cells = append(cells, row)
	}
	table := render.Table(cells, 0, 2)
	buf.WriteString(table)
	return buf.String()
}

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func RenderChips(chips int) string {
	return render.Fg(render.Green, strconv.Itoa(chips))
}

func RenderCard(card int) string {
	return render.Fg(render.Blue, strconv.Itoa(card))
}

func RenderPoints(points int) string {
	return render.Fg(render.Purple, strconv.Itoa(points))
}

func (g *Game) RenderCardsForPlayer(player int, relevant int) string {
	renderGroups := []string{}
	for _, group := range g.PlayerHandGrouped(player) {
		renderGroup := []string{}
		for _, c := range group {
			renderedCard := RenderCard(c)
			if c-relevant == 1 || c-relevant == -1 {
				renderedCard = render.Bold(renderedCard)
			}
			renderGroup = append(renderGroup, renderedCard)
		}
		renderGroups = append(renderGroups, strings.Join(renderGroup, " "))
	}
	return strings.Join(renderGroups, "   ")
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 3 || players > 5 {
		return nil, errors.New("No Thanks requires between 3 and 5 players")
	}
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	g.Players = players
	g.InitCards()
	g.InitPlayerChips()
	g.InitPlayerHands()
	g.CurrentlyMoving = r.Int() % g.Players
	return nil, nil
}

func (g *Game) IsFinished() bool {
	return len(g.RemainingCards) == 0
}

func (g *Game) PointsInt() []int {
	points := make([]int, g.Players)
	for p := 0; p < g.Players; p++ {
		if g.IsFinished() {
			points[p] = g.FinalPlayerScore(p)
		} else {
			points[p] = g.PlayerHandScore(p)
		}
	}
	return points
}

func (g *Game) Points() []float32 {
	points := make([]float32, g.Players)
	for p, pts := range g.PointsInt() {
		points[p] = float32(pts)
	}
	return points
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p, pts := range g.PointsInt() {
		metrics[p] = []int{-pts}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentlyMoving}
}

func AllCards() []int {
	cards := make([]int, 33)
	for i := 3; i <= 35; i++ {
		cards[i-3] = i
	}
	return cards
}

func (g *Game) InitCards() {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	perm := r.Perm(33)
	cardPool := AllCards()
	g.RemainingCards = make([]int, 24)
	for i := 0; i < 24; i++ {
		g.RemainingCards[i] = cardPool[perm[i]]
	}
}

func (g *Game) InitPlayerChips() {
	g.PlayerChips = make([]int, g.Players)
	for p := 0; p < g.Players; p++ {
		g.PlayerChips[p] = 11
	}
}

func (g *Game) InitPlayerHands() {
	g.PlayerHands = make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		g.PlayerHands[p] = []int{}
	}
}

func (g *Game) Pass(player int) ([]brdgme.Log, error) {
	if !g.CanPass(player) {
		return nil, errors.New("can't pass at the moment")
	}
	if g.PlayerChips[player] <= 0 {
		return nil, errors.New("You have no chips left, you must take the card")
	}
	g.PlayerChips[player]--
	g.CentreChips++
	g.NextPlayer()
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s passed on the %s",
		render.Player(player),
		render.Bold(RenderCard(g.PeekTopCard())),
	))}, nil
}

func (g *Game) Take(player int) ([]brdgme.Log, error) {
	if !g.CanTake(player) {
		return nil, errors.New("can't take at the moment")
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s took the %s and %s chips",
		render.Player(player),
		render.Bold(RenderCard(g.PeekTopCard())),
		render.Bold(RenderChips(g.CentreChips)),
	))}
	g.PlayerHands[player] = append(g.PlayerHands[player], g.PopTopCard())
	if !g.IsFinished() {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s drew %s as the new card",
			render.Player(player),
			render.Bold(RenderCard(g.PeekTopCard())),
		)))
	}
	g.PlayerChips[player] += g.CentreChips
	g.CentreChips = 0
	return logs, nil
}

func (g *Game) PeekTopCard() int {
	if len(g.RemainingCards) == 0 {
		panic("no cards remaining")
	}
	return g.RemainingCards[len(g.RemainingCards)-1]
}

func (g *Game) PopTopCard() int {
	top := g.PeekTopCard()
	g.RemainingCards = g.RemainingCards[:len(g.RemainingCards)-1]
	return top
}

func (g *Game) NextPlayer() {
	g.CurrentlyMoving = (g.CurrentlyMoving + 1) % g.Players
}

func (g *Game) PlayerHandSorted(player int) []int {
	sort.Ints(g.PlayerHands[player])
	return g.PlayerHands[player]
}

func (g *Game) PlayerHandGrouped(player int) [][]int {
	groups := [][]int{}
	curGroup := []int{}
	lastCard := -1
	for _, c := range g.PlayerHandSorted(player) {
		if c == lastCard+1 {
			curGroup = append(curGroup, c)
		} else {
			if len(curGroup) > 0 {
				groups = append(groups, curGroup)
			}
			curGroup = []int{c}
		}
		lastCard = c
	}
	if len(curGroup) > 0 {
		groups = append(groups, curGroup)
	}
	return groups
}

func (g *Game) PlayerHandScore(player int) int {
	score := 0
	for _, g := range g.PlayerHandGrouped(player) {
		score += g[0]
	}
	return score
}

func (g *Game) FinalPlayerScore(player int) int {
	return g.PlayerHandScore(player) - g.PlayerChips[player]
}

func (g *Game) CanPass(player int) bool {
	return g.CurrentlyMoving == player && g.PlayerChips[player] > 0 &&
		!g.IsFinished()
}

func (g *Game) CanTake(player int) bool {
	return g.CurrentlyMoving == player && !g.IsFinished()
}
