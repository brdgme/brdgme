package for_sale_1

import (
	"bytes"
	"errors"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	card "github.com/brdgme/brdgme/brdgme-go/libcard"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	BuyingPhase  = 0
	SellingPhase = 1
	GameFinished = 2
)

type Game struct {
	Players         int
	BuildingDeck    card.Deck
	ChequeDeck      card.Deck
	OpenCards       card.Deck
	Hands           map[int]card.Deck
	Cheques         map[int]card.Deck
	Chips           map[int]int
	BiddingPlayer   int
	Bids            map[int]int
	FinishedBidding map[int]bool
}

var _ brdgme.Gamer = &Game{}

type PubState struct {
	Players             int
	BuyRoundsRemaining  int
	SellRoundsRemaining int
	OpenCards           card.Deck
	BiddingPlayer       int
	Bids                map[int]int
	FinishedBidding     map[int]bool
}

type PlayerState struct {
	PubState PubState
	Player   int
	Hand     card.Deck
	Cheques  card.Deck
	Chips    int
}

func (g *Game) ToPubState() PubState {
	return PubState{
		Players:             g.Players,
		BuyRoundsRemaining:  g.BuildingDeck.Len() / g.Players,
		SellRoundsRemaining: g.ChequeDeck.Len() / g.Players,
		OpenCards:           g.OpenCards,
		BiddingPlayer:       g.BiddingPlayer,
		Bids:                g.Bids,
		FinishedBidding:     g.FinishedBidding,
	}
}

func (g *Game) ToPlayerState(player int) PlayerState {
	return PlayerState{
		PubState: g.ToPubState(),
		Player:   player,
		Hand:     g.Hands[player],
		Cheques:  g.Cheques[player],
		Chips:    g.Chips[player],
	}
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	logs := []brdgme.Log{}
	if players < 3 || players > 5 {
		return logs, errors.New("must have between 3 and 5 players")
	}
	g.Players = players
	g.BuildingDeck = BuildingDeck().Shuffle()
	g.ChequeDeck = ChequeDeck().Shuffle()
	g.Hands = map[int]card.Deck{}
	g.Cheques = map[int]card.Deck{}
	g.Chips = map[int]int{}
	g.Bids = map[int]int{}
	g.FinishedBidding = map[int]bool{}
	for p := 0; p < g.Players; p++ {
		g.Hands[p] = card.Deck{}
		g.Cheques[p] = card.Deck{}
		g.Chips[p] = 15
		g.Bids[p] = 0
		g.FinishedBidding[p] = false
	}
	if players == 3 {
		logs = append(logs, brdgme.NewPublicLog(
			"Removing two building and cheque cards for 3 player game",
		))
		_, g.BuildingDeck = g.BuildingDeck.PopN(2)
		_, g.ChequeDeck = g.ChequeDeck.PopN(2)
	}
	logs = append(logs, g.StartRound()...)
	return logs, nil
}

func (g *Game) CurrentPhase() int {
	if len(g.BuildingDeck) > 0 ||
		(len(g.OpenCards) > 0 && len(g.ChequeDeck) >= 18) {
		return BuyingPhase
	} else if len(g.ChequeDeck) > 0 || len(g.OpenCards) > 0 {
		return SellingPhase
	}
	return GameFinished
}

func (g *Game) PubState() interface{} {
	return g.ToPubState()
}

func (g *Game) PlayerState(player int) interface{} {
	return g.ToPlayerState(player)
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

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{3, 4, 5}
}

func (g *Game) StartRound() []brdgme.Log {
	switch g.CurrentPhase() {
	case BuyingPhase:
		return g.StartBuyingRound()
	case SellingPhase:
		return g.StartSellingRound()
	case GameFinished:
		output := bytes.NewBufferString(
			render.Bold("The game has finished!  The scores are:"),
		)
		output.WriteString("\n")
		playerScores := [][]render.Cell{}
		for pNum := 0; pNum < g.Players; pNum++ {
			playerScores = append(playerScores, []render.Cell{
				{
					Align:   render.Left,
					Content: render.Player(pNum),
				},
				{
					Align:   render.Left,
					Content: render.Bold(strconv.Itoa(g.DeckValue(g.Cheques[pNum]))),
				},
			})
		}
		table := render.Table(playerScores, 0, 1)
		output.WriteString(table)
		return []brdgme.Log{brdgme.NewPublicLog(output.String())}
	}
	return []brdgme.Log{}
}

func (g *Game) StartBuyingRound() []brdgme.Log {
	g.OpenCards, g.BuildingDeck = g.BuildingDeck.PopN(g.Players)
	g.OpenCards = g.OpenCards.Sort()
	g.ClearBids()
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`Drew new buildings: %s`,
		strings.Join(RenderCards(g.OpenCards, RenderBuilding), " "),
	))}
}

func (g *Game) StartSellingRound() []brdgme.Log {
	g.OpenCards, g.ChequeDeck = g.ChequeDeck.PopN(g.Players)
	g.OpenCards = g.OpenCards.Sort()
	g.ClearBids()
	logs := []brdgme.Log{}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		`Drew new cheques: %s`,
		strings.Join(RenderCards(g.OpenCards, RenderCheque), " "),
	)))
	if g.Hands[0].Len() == 1 {
		// Autoplay the final card
		for p := 0; p < g.Players; p++ {
			playLogs, _ := g.Play(p, g.Hands[p][0].Rank)
			logs = append(logs, playLogs...)
		}
	}
	return logs
}

func (g *Game) ClearBids() {
	for p := 0; p < g.Players; p++ {
		g.Bids[p] = 0
		g.FinishedBidding[p] = false
	}
}

func (g *Game) DeckValue(deck card.Deck) int {
	value := 0
	for _, c := range deck {
		value += c.Rank
	}
	return value
}

func (g *Game) IsFinished() bool {
	return len(g.OpenCards) == 0 && len(g.BuildingDeck) == 0 &&
		len(g.ChequeDeck) == 0
}

func (g *Game) Placings() []int {
	metrics := [][]int{}
	for p := 0; p < g.Players; p++ {
		metrics = append(metrics, []int{
			g.PlayerPoints(p),
			g.Chips[p],
		})
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	if g.CurrentPhase() == BuyingPhase {
		return g.WhoseTurnBuying()
	}
	return g.WhoseTurnSelling()
}

func (g *Game) WhoseTurnBuying() []int {
	return []int{g.BiddingPlayer}
}

func (g *Game) WhoseTurnSelling() []int {
	players := []int{}
	for p := 0; p < g.Players; p++ {
		if !g.FinishedBidding[p] {
			players = append(players, p)
		}
	}
	return players
}

func (g *Game) CanBid(player int) bool {
	return g.CurrentPhase() == BuyingPhase &&
		g.BiddingPlayer == player
}

func (g *Game) Bid(player, amount int) ([]brdgme.Log, error) {
	logs := []brdgme.Log{}
	if !g.CanBid(player) {
		return logs, errors.New("you are not able to bid at the moment")
	}
	if amount > g.Chips[player] {
		return logs, fmt.Errorf(
			"cannot bid %d, you only have %d",
			amount,
			g.Chips[player],
		)
	}
	if _, highest := g.HighestBid(); amount <= highest {
		return logs, fmt.Errorf("you must bid higher than %d", highest)
	}
	g.Bids[player] = amount
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s bid %s",
		render.Player(player),
		render.Bold(strconv.Itoa(amount)),
	)))
	logs = append(logs, g.NextBidder()...)
	return logs, nil
}

func (g *Game) CanPass(player int) bool {
	return g.CanBid(player)
}

func (g *Game) Pass(player int) ([]brdgme.Log, error) {
	logs := []brdgme.Log{}
	if !g.CanPass(player) {
		return logs, errors.New("you are not able to pass at the moment")
	}
	c := g.TakeFirstOpenCard(player)
	halfBid := g.Bids[player] / 2
	g.Chips[player] -= halfBid
	g.FinishedBidding[player] = true
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s passed, paying %s for %s",
		render.Player(player),
		render.Bold(strconv.Itoa(halfBid)),
		RenderBuilding(c.Rank),
	)))
	logs = append(logs, g.NextBidder()...)
	return logs, nil
}

func (g *Game) CanPlay(player int) bool {
	return g.CurrentPhase() == SellingPhase &&
		player < g.Players &&
		!g.FinishedBidding[player]
}

func (g *Game) Play(player, building int) ([]brdgme.Log, error) {
	var cheque card.Card
	logs := []brdgme.Log{}
	if !g.CanPlay(player) {
		return logs, errors.New("you are not able to play a building card at the moment")
	}
	remaining, n := g.Hands[player].Remove(card.Card{
		Rank: building,
	}, 1)
	if n == 0 {
		return logs, errors.New("you don't have that card in your hand")
	}
	g.Hands[player] = remaining
	g.Bids[player] = building
	g.FinishedBidding[player] = true
	if len(g.WhoseTurn()) == 0 {
		played := card.Deck{}
		for p, b := range g.Bids {
			played = append(played, card.Card{
				Suit: b,
				Rank: p,
			})
		}
		for _, c := range played.Sort() {
			p := c.Rank
			cheque, g.OpenCards = g.OpenCards.Shift()
			g.Cheques[p] = g.Cheques[p].Push(cheque)
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`%s sold %s for %s`,
				render.Player(p),
				RenderBuilding(c.Suit),
				RenderCheque(cheque.Rank),
			)))
		}
		logs = append(logs, g.StartRound()...)
	}
	return logs, nil
}

func (g *Game) TakeFirstOpenCard(player int) card.Card {
	var c card.Card
	c, g.OpenCards = g.OpenCards.Shift()
	g.Hands[player] = g.Hands[player].Push(c).Sort()
	return c
}

func (g *Game) NextBidder() []brdgme.Log {
	remaining := 0
	for _, b := range g.FinishedBidding {
		if !b {
			remaining++
		}
	}
	logs := []brdgme.Log{}
	if remaining == 1 {
		// Last remaining player takes the last building for the full price.
		player, amount := g.HighestBid()
		c := g.TakeFirstOpenCard(player)
		g.Chips[player] -= amount
		g.BiddingPlayer = player
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s is the last player, paying %s for %s",
			render.Player(player),
			render.Bold(strconv.Itoa(amount)),
			RenderBuilding(c.Rank),
		)))
		logs = append(logs, g.StartRound()...)
		return logs
	}
	for {
		g.BiddingPlayer = (g.BiddingPlayer + 1) % g.Players
		if !g.FinishedBidding[g.BiddingPlayer] {
			break
		}
	}
	return logs
}

func (g *Game) HighestBid() (player, amount int) {
	amount = -1
	for p, b := range g.Bids {
		if !g.FinishedBidding[p] && b > amount {
			player = p
			amount = b
		}
	}
	return
}

func (g *Game) PlayerPoints(player int) int {
	return g.DeckValue(g.Cheques[player]) + g.Chips[player]
}

func (g *Game) Points() []float32 {
	points := make([]float32, g.Players)
	isFinished := g.IsFinished()
	for p := 0; p < g.Players; p++ {
		if isFinished {
			points[p] = float32(g.PlayerPoints(p))
		} else {
			points[p] = 0
		}
	}
	return points
}

func BuildingDeck() card.Deck {
	d := card.Deck{}
	for i := 1; i <= 20; i++ {
		d = d.Push(card.Card{
			Rank: i,
		})
	}
	return d
}

func ChequeDeck() card.Deck {
	d := card.Deck{}
	for i := 1; i <= 20; i++ {
		c := card.Card{
			Rank: i,
		}
		if i < 3 {
			c.Rank = 0
		}
		d = d.Push(c)
	}
	return d
}

func RenderBuilding(value int) string {
	return render.Bold(render.Fg(render.Green, strconv.Itoa(value)))
}

func RenderCheque(value int) string {
	return render.Bold(render.Fg(render.Blue, strconv.Itoa(value)))
}

func RenderCards(deck card.Deck, renderer func(int) string) []string {
	output := []string{}
	for _, c := range deck {
		output = append(output, renderer(c.Rank))
	}
	return output
}
