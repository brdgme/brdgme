package love_letter

import (
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

type Game struct {
	Players         int
	Round           int
	Deck, Removed   []int
	Hands, Discards [][]int
	PlayerPoints    []int
	CurrentPlayer   int
	Eliminated      []bool
	Protected       []bool
}

var _ brdgme.Gamer = &Game{}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 4 {
		return nil, errors.New("only for 2 to 4 players")
	}
	g.Players = players
	g.PlayerPoints = make([]int, players)
	return g.StartRound(), nil
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
		points[p] = float32(g.PlayerPoints[p])
	}
	return points
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{g.PlayerPoints[p]}
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

func (g *Game) StartRound() []brdgme.Log {
	logs := []brdgme.Log{}
	g.Round++
	g.Eliminated = make([]bool, g.Players)
	g.Protected = make([]bool, g.Players)
	deck := brdgme.IntShuffle(Deck)
	remove := 1
	if g.Players == 2 {
		remove = 4
	}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"Starting round %d, {{b}}removing %d %s{{/b}}",
		g.Round,
		remove,
		brdgme.Plural(remove, "card"),
	)))
	g.Deck, g.Removed = deck[remove:], deck[:remove]
	g.Hands = make([][]int, g.Players)
	g.Discards = make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		g.Hands[p] = []int{}
		g.Discards[p] = []int{}
		logs = append(logs, g.DrawCard(p)...)
	}
	logs = append(logs, g.StartTurn()...)
	return logs
}

func (g *Game) StartTurn() []brdgme.Log {
	g.Protected[g.CurrentPlayer] = false
	if len(g.Deck) == 0 {
		return g.EndRound()
	} else {
		return g.DrawCard(g.CurrentPlayer)
	}
}

func (g *Game) NextPlayer() []brdgme.Log {
	for {
		g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players
		if !g.Eliminated[g.CurrentPlayer] {
			break
		}
	}
	return g.StartTurn()
}

func (g *Game) DiscardCardLog(player, card int) []brdgme.Log {
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s discarded %s",
		render.Player(player),
		RenderCard(card),
	))}
	logs = append(logs, g.DiscardCard(player, card)...)
	return logs
}

func (g *Game) DiscardCard(player, card int) []brdgme.Log {
	g.Hands[player] = brdgme.IntRemove(card, g.Hands[player], 1)
	g.Discards[player] = append(g.Discards[player], card)
	if card == Princess {
		return g.Eliminate(player)
	}
	return nil
}

func (g *Game) Eliminate(player int) []brdgme.Log {
	if g.Eliminated[player] {
		return nil
	}

	g.Eliminated[player] = true
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s has been eliminated from this round",
		render.Player(player),
	))}
	for len(g.Hands[player]) > 0 {
		logs = append(logs, g.DiscardCardLog(player, g.Hands[player][0])...)
	}

	numRemaining := 0
	for p := 0; p < g.Players; p++ {
		if !g.Eliminated[p] {
			numRemaining++
		}
	}
	if numRemaining <= 1 {
		logs = append(logs, g.EndRound()...)
	}
	return logs
}

func (g *Game) EndRound() []brdgme.Log {
	output := []string{render.Bold("It is the end of the round")}
	var highestCard, highestPlayer, discardTotal int
	for p := 0; p < g.Players; p++ {
		if g.Eliminated[p] {
			continue
		}
		c := g.Hands[p][0]
		discarded := brdgme.IntSum(g.Discards[p])
		output = append(output, fmt.Sprintf(
			"%s had %s (total {{b}}%d{{/b}} discarded)",
			render.Player(p),
			RenderCard(c),
			discarded,
		))
		if c > highestCard {
			highestCard = c
			discardTotal = -1
		}
		if c == highestCard {
			if discarded > discardTotal {
				discardTotal = discarded
				highestPlayer = p
			}
		}
	}

	g.PlayerPoints[highestPlayer]++
	output = append(output, fmt.Sprintf(
		"%s won the round and moved to {{b}}%d %s{{/b}}",
		render.Player(highestPlayer),
		g.PlayerPoints[highestPlayer],
		brdgme.Plural(g.PlayerPoints[highestPlayer], "point"),
	))

	isFinished := g.IsFinished()
	if isFinished {
		output = append(output, render.Bold(fmt.Sprintf(
			"It is the end of the game, the winner is %s",
			render.Player(g.Leader()),
		)))
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(strings.Join(output, "\n"))}
	if !isFinished {
		g.CurrentPlayer = highestPlayer
		logs = append(logs, g.StartRound()...)
	}
	return logs
}

func (g *Game) Leader() int {
	var highest, player int
	for p := 0; p < g.Players; p++ {
		points := g.PlayerPoints[p]
		if points > highest {
			player = p
			highest = points
		}
	}
	return player
}

func (g *Game) DrawCard(player int) []brdgme.Log {
	logs := []brdgme.Log{}
	var card int
	if len(g.Deck) > 0 {
		card, g.Deck = g.Deck[0], g.Deck[1:]
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s drew a card from the draw pile, {{b}}%d{{/b}} remaining",
			render.Player(player),
			len(g.Deck),
		)))
	} else {
		card = g.Removed[0]
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s drew a card from the removed cards",
			render.Player(player),
		)))
	}
	logs = append(logs, brdgme.NewPrivateLog(fmt.Sprintf(
		"You drew %s",
		RenderCard(card),
	), []int{player}))
	g.Hands[player] = append(g.Hands[player], card)
	return logs
}

var endScores = map[int]int{
	2: 7,
	3: 5,
	4: 4,
}

func (g *Game) IsFinished() bool {
	return brdgme.IntMax(g.PlayerPoints...) >= endScores[g.Players]
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) AvailableTargets(forPlayer int) []int {
	targets := []int{}
	for p := 0; p < g.Players; p++ {
		if p != forPlayer && !g.Eliminated[p] && !g.Protected[p] {
			targets = append(targets, p)
		}
	}
	return targets
}

func (g *Game) AssertTarget(player int, incSelf bool, target int) error {
	availableTargets := g.AvailableTargets(player)
	if len(availableTargets) == 0 {
		if target == player {
			return nil
		}
		return errors.New("all other players are protected by the Handmaid, so you must target yourself")
	}

	if !incSelf && target == player {
		return errors.New("you cannot target yourself if there are other players you can target")
	}

	if g.Eliminated[target] {
		return errors.New("that player is eliminated")
	}
	if g.Eliminated[target] {
		return errors.New("that player is protected by the Handmaid")
	}

	return nil
}

func (g *Game) CanPlay(player int) bool {
	return g.CurrentPlayer == player
}
