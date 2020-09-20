package texas_holdem

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"strings"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
	"github.com/brdgme/brdgme/brdgme-go/libpoker"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	STARTING_MONEY            = 100
	STARTING_MINIMUM_BET      = 10
	HANDS_PER_BLINDS_INCREASE = 5
)

var _ brdgme.Gamer = &Game{}

type Game struct {
	Players                  int
	CurrentPlayer            int
	CurrentDealer            int
	PlayerHands              []libcard.Deck
	CommunityCards           libcard.Deck
	Deck                     libcard.Deck
	PlayerMoney              []int
	Bets                     []int
	FoldedPlayers            []bool
	MinimumBet               int
	LargestRaise             int
	HandsSinceBlindsIncrease int
	FirstBettingPlayer       int
	EveryoneHasBetOnce       bool
}

func RenderCash(amount int) string {
	return render.Markup(fmt.Sprintf("$%d", amount), render.Green, true)
}

func RenderCashFixedWidth(amount int) string {
	output := RenderCash(amount)
	if amount < 10 {
		output += " "
	}
	if amount < 100 {
		output += " "
	}
	return output
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 9 {
		return nil, errors.New("Texas hold 'em is limited to 2 - 9 players")
	}
	g.Players = players
	g.PlayerHands = make([]libcard.Deck, g.Players)
	g.PlayerMoney = make([]int, g.Players)
	for i := 0; i < g.Players; i++ {
		g.PlayerMoney[i] = STARTING_MONEY
	}
	g.MinimumBet = STARTING_MINIMUM_BET
	// Pick a random starting player
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	g.CurrentDealer = r.Int() % g.Players
	return g.NewHand(), nil
}

func (g *Game) NewHand() []brdgme.Log {
	var (
		smallBlindPlayer, bigBlindPlayer int
	)
	// Reset values
	g.FoldedPlayers = make([]bool, g.Players)
	g.Bets = make([]int, g.Players)
	g.LargestRaise = 0
	g.EveryoneHasBetOnce = false
	g.NewBettingRound()
	activePlayers := g.ActivePlayers()
	numActivePlayers := len(activePlayers)
	logs := []brdgme.Log{}
	// Raise blinds if we need to
	if g.HandsSinceBlindsIncrease >= HANDS_PER_BLINDS_INCREASE {
		g.HandsSinceBlindsIncrease = 0
		g.MinimumBet *= 2
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Minimum bet increased to %s", RenderCash(g.MinimumBet))))
	} else {
		g.HandsSinceBlindsIncrease += 1
	}
	// Set a new active dealer
	g.CurrentDealer = g.NextActivePlayerNumFrom(g.CurrentDealer)
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf("%s is the new dealer",
		g.RenderPlayerName(g.CurrentDealer))))
	// Blinds
	if numActivePlayers == 2 {
		// Special head-to-head rules for 2 player
		// @see https://en.wikipedia.org/wiki/Texas_hold_'em#Betting_structures
		smallBlindPlayer = g.CurrentDealer
	} else {
		smallBlindPlayer = g.NextActivePlayerNumFrom(g.CurrentDealer)
	}
	amount := g.BetUpTo(smallBlindPlayer, g.MinimumBet/2)
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s posted a small blind of %s", g.RenderPlayerName(smallBlindPlayer),
		RenderCash(amount))))
	bigBlindPlayer = g.NextActivePlayerNumFrom(smallBlindPlayer)
	amount = g.BetUpTo(bigBlindPlayer, g.MinimumBet)
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s posted a big blind of %s", g.RenderPlayerName(bigBlindPlayer),
		RenderCash(amount))))
	// Shuffle and deal two cards to each player
	g.CommunityCards = libcard.Deck{}
	g.Deck = libcard.Standard52DeckAceHigh().Shuffle()
	for i, _ := range activePlayers {
		g.PlayerHands[i], g.Deck = g.Deck.PopN(2)
		g.PlayerHands[i] = g.PlayerHands[i].Sort()
	}
	if len(g.BettingPlayers()) > 0 {
		// Make the current player the one next to the big blind
		g.CurrentPlayer = g.NextBettingPlayerNumFrom(bigBlindPlayer)
		g.FirstBettingPlayer = g.CurrentPlayer
	} else {
		// Nobody has money!  Just go to next phase.
		logs = append(logs, g.NextPhase()...)
	}
	return logs
}

// Remaining players who haven't busted yet
func (g *Game) RemainingPlayers() []int {
	remaining := []int{}
	for p := 0; p < g.Players; p++ {
		if g.PlayerMoney[p] > 0 || g.Bets[p] > 0 {
			remaining = append(remaining, p)
		}
	}
	return remaining
}

// Active players are players who are still in the game and haven't folded
func (g *Game) ActivePlayers() []int {
	active := []int{}
	for _, p := range g.RemainingPlayers() {
		if !g.FoldedPlayers[p] {
			active = append(active, p)
		}
	}
	return active
}

// Betting players are active players who still have money
func (g *Game) BettingPlayers() []int {
	betting := []int{}
	for _, p := range g.ActivePlayers() {
		if g.PlayerMoney[p] > 0 {
			betting = append(betting, p)
		}
	}
	return betting
}

// Requiring call players are betting players who are behind the current bet
func (g *Game) RequiringCallPlayers() []int {
	requiringCall := []int{}
	currentBet := g.CurrentBet()
	for _, p := range g.BettingPlayers() {
		if g.Bets[p] < currentBet {
			requiringCall = append(requiringCall, p)
		}
	}
	return requiringCall
}

func (g *Game) NextActivePlayerNumFrom(playerNum int) int {
	return g.NextPlayerInSet(playerNum, g.ActivePlayers())
}

func (g *Game) NextBettingPlayerNumFrom(playerNum int) int {
	return g.NextPlayerInSet(playerNum, g.BettingPlayers())
}

func (g *Game) NextRemainingPlayerNumFrom(playerNum int) int {
	return g.NextPlayerInSet(playerNum, g.RemainingPlayers())
}

func (g *Game) NextPlayerInSet(playerNum int, set []int) int {
	if len(set) == 0 {
		panic("No players in set")
	}
	for i := 0; i < g.Players; i++ {
		nextPlayerNum := (playerNum + i + 1) % g.Players
		if _, ok := brdgme.IntFind(nextPlayerNum, set); ok {
			return nextPlayerNum
		}
	}
	panic("Could not find any valid players")
}

func (g *Game) BetUpTo(playerNum int, amount int) int {
	betAmount := min(amount, g.PlayerMoney[playerNum])
	err := g.Bet(playerNum, betAmount)
	if err != nil {
		panic(err.Error())
	}
	return betAmount
}

func (g *Game) Bet(playerNum int, amount int) error {
	if g.PlayerMoney[playerNum] < amount {
		return errors.New("Not enough money")
	}
	raiseAmount := g.Bets[playerNum] + amount - g.CurrentBet()
	g.Bets[playerNum] += amount
	g.PlayerMoney[playerNum] -= amount
	g.LargestRaise = max(raiseAmount, g.LargestRaise)
	return nil
}

func (g *Game) Check(playerNum int) ([]brdgme.Log, error) {
	if g.IsFinished() || g.CurrentPlayer != playerNum {
		return nil, errors.New("Not your turn")
	}
	if g.CurrentBet() > g.Bets[playerNum] {
		return nil, errors.New("Cannot check because you are below the bet")
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf("%s checked",
		g.RenderPlayerName(playerNum)))}
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanCheck(player int) bool {
	currentBet := g.CurrentBet()
	return g.CurrentPlayer == player && g.Bets[player] == currentBet &&
		!g.IsFinished()
}

func (g *Game) Fold(playerNum int) ([]brdgme.Log, error) {
	if g.IsFinished() || g.CurrentPlayer != playerNum {
		return nil, errors.New("Not your turn")
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf("%s folded",
		g.RenderPlayerName(playerNum)))}
	g.FoldedPlayers[playerNum] = true
	if len(g.ActivePlayers()) == 1 {
		// Everyone folded
		for activePlayerNum, _ := range g.ActivePlayers() {
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				"%s took %s", g.RenderPlayerName(activePlayerNum),
				RenderCash(g.Pot()))))
			g.PlayerMoney[activePlayerNum] += g.Pot()
			logs = append(logs, g.NewHand()...)
			return logs, nil
		}
	} else {
		logs = append(logs, g.NextPlayer()...)
	}
	return logs, nil
}

func (g *Game) CanFold(player int) bool {
	currentBet := g.CurrentBet()
	return g.CurrentPlayer == player && g.Bets[player] < currentBet &&
		!g.IsFinished()
}

func (g *Game) Call(playerNum int) ([]brdgme.Log, error) {
	if g.IsFinished() || g.CurrentPlayer != playerNum {
		return nil, errors.New("Not your turn")
	}
	difference := g.CurrentBet() - g.Bets[playerNum]
	if g.PlayerMoney[playerNum] < difference {
		return nil, errors.New("You don't have enough to call, you can only go allin")
	}
	if difference <= 0 {
		return nil, errors.New(
			"You are already at the current bet, you may check if you don't want to raise")
	}
	err := g.Bet(playerNum, difference)
	if err != nil {
		return nil, err
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf("%s called",
		g.RenderPlayerName(playerNum)))}
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanCall(player int) bool {
	currentBet := g.CurrentBet()
	return g.CurrentPlayer == player && g.Bets[player] < currentBet &&
		g.PlayerMoney[player] > currentBet-g.Bets[player] &&
		!g.IsFinished()
}

func (g *Game) MinRaise() int {
	return max(g.MinimumBet, g.LargestRaise)
}

func (g *Game) Raise(playerNum int, amount int) ([]brdgme.Log, error) {
	if g.IsFinished() || g.CurrentPlayer != playerNum {
		return nil, errors.New("Not your turn")
	}
	minRaise := g.MinRaise()
	difference := g.CurrentBet() - g.Bets[playerNum]
	if amount < minRaise {
		return nil, errors.New(fmt.Sprintf(
			"Your raise must be at least %d", minRaise))
	}
	err := g.Bet(playerNum, difference+amount)
	if err != nil {
		return nil, err
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf("%s raised by %s",
		g.RenderPlayerName(playerNum), RenderCash(amount)))}
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanRaise(player int) bool {
	currentBet := g.CurrentBet()
	minRaise := g.LargestRaise
	return g.CurrentPlayer == player &&
		g.PlayerMoney[player] > currentBet-g.Bets[player]+minRaise &&
		!g.IsFinished()
}

func (g *Game) AllIn(playerNum int) ([]brdgme.Log, error) {
	if g.IsFinished() || g.CurrentPlayer != playerNum {
		return nil, errors.New("Not your turn")
	}
	amount := g.PlayerMoney[playerNum]
	err := g.Bet(playerNum, amount)
	if err != nil {
		return nil, err
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf("%s went all in with %s",
		g.RenderPlayerName(playerNum), RenderCash(amount)))}
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanAllIn(player int) bool {
	return g.CurrentPlayer == player && g.PlayerMoney[player] > 0 &&
		!g.IsFinished()
}

func (g *Game) NextPlayer() []brdgme.Log {
	logs := []brdgme.Log{}
	requiringCallPlayers := g.RequiringCallPlayers()
	bettingPlayers := g.BettingPlayers()
	if len(bettingPlayers) > 0 {
		nextPlayer := g.NextPlayerInSet(g.CurrentPlayer, bettingPlayers)
		if !g.EveryoneHasBetOnce {
			// Check if we've passed the first fplayer
			distanceToFirst := g.FirstBettingPlayer - g.CurrentPlayer
			if distanceToFirst <= 0 {
				distanceToFirst += g.Players
			}
			distanceToNextPlayer := nextPlayer - g.CurrentPlayer
			if distanceToNextPlayer <= 0 {
				distanceToNextPlayer += g.Players
			}
			if distanceToNextPlayer >= distanceToFirst {
				g.EveryoneHasBetOnce = true
			}
		}
		if len(requiringCallPlayers) == 0 && g.EveryoneHasBetOnce {
			logs = append(logs, g.NextPhase()...)
		} else {
			g.CurrentPlayer = nextPlayer
		}
	} else {
		logs = append(logs, g.NextPhase()...)
	}
	return logs
}

func (g *Game) NextPhase() []brdgme.Log {
	logs := []brdgme.Log{}
	bettingPlayersCount := len(g.BettingPlayers())
	switch len(g.CommunityCards) {
	case 0:
		logs = append(logs, g.Flop()...)
		if bettingPlayersCount < 2 {
			logs = append(logs, g.NextPhase()...)
		}
	case 3:
		logs = append(logs, g.Turn()...)
		if bettingPlayersCount < 2 {
			logs = append(logs, g.NextPhase()...)
		}
	case 4:
		logs = append(logs, g.River()...)
		if bettingPlayersCount < 2 {
			logs = append(logs, g.NextPhase()...)
		}
	case 5:
		logs = append(logs, g.Showdown()...)
	}
	return logs
}

func (g *Game) Flop() []brdgme.Log {
	g.NewCommunityCards(3)
	logs := []brdgme.Log{brdgme.NewPublicLog("Flop cards are " +
		render.Bold(strings.Join(RenderCards(g.CommunityCards), " ")))}
	g.NewBettingRound()
	return logs
}

func (g *Game) Turn() []brdgme.Log {
	g.NewCommunityCards(1)
	logs := []brdgme.Log{brdgme.NewPublicLog("Turn card is " +
		render.Bold(g.CommunityCards[3].RenderStandard52()))}
	g.NewBettingRound()
	return logs
}

func (g *Game) River() []brdgme.Log {
	g.NewCommunityCards(1)
	logs := []brdgme.Log{brdgme.NewPublicLog("River card is " +
		render.Bold(g.CommunityCards[4].RenderStandard52()))}
	g.NewBettingRound()
	return logs
}

func (g *Game) Showdown() []brdgme.Log {
	buf := bytes.NewBufferString(render.Bold("Showdown\n"))
	for g.Pot() > 0 {
		// Find the minimum bet
		smallest := g.SmallestBet()
		pot := 0
		handResults := map[int]libpoker.HandResult{}
		handsTable := [][]render.Cell{}
		for playerNum, b := range g.Bets {
			if b == 0 {
				continue
			}
			contribution := min(b, smallest)
			pot += contribution
			g.Bets[playerNum] -= contribution
			if !g.FoldedPlayers[playerNum] {
				handResults[playerNum] = libpoker.Result(
					g.PlayerHands[playerNum].PushMany(g.CommunityCards))
				handsTableRow := []render.Cell{render.Cel(g.RenderPlayerName(playerNum))}
				handsTableRow = append(handsTableRow, render.Cel(strings.Join(
					RenderCards(g.PlayerHands[playerNum]), " ")))
				handsTableRow = append(handsTableRow,
					render.Cel(handResults[playerNum].Name))
				handsTableRow = append(handsTableRow, render.Cel(strings.Join(
					RenderCards(handResults[playerNum].Cards), " ")))
				handsTable = append(handsTable, handsTableRow)
			}
		}
		if len(handResults) > 1 {
			// Multiple people for this pot, showdown
			handsTableOutput := render.Table(handsTable, 0, 2)
			buf.WriteString(fmt.Sprintf("Showdown for pot of %s\n%s\n",
				RenderCash(pot), handsTableOutput))
			winners := libpoker.WinningHandResult(handResults)
			potPerPlayer := pot / len(winners)
			for _, winner := range winners {
				buf.WriteString(fmt.Sprintf("%s took %s (%s)\n",
					g.RenderPlayerName(winner), RenderCash(potPerPlayer),
					handResults[winner].Name))
				g.PlayerMoney[winner] += potPerPlayer
			}
			remainder := pot - potPerPlayer*len(winners)
			if remainder > 0 {
				remainderPlayer := g.NextRemainingPlayerNumFrom(g.CurrentDealer)
				buf.WriteString(fmt.Sprintf("%s took %s due to uneven split",
					g.RenderPlayerName(remainderPlayer), RenderCash(remainder)))
				g.PlayerMoney[remainderPlayer] += remainder
			}
		} else {
			// Only one player left for the pot, give it to them
			for playerNum, handResult := range handResults {
				buf.WriteString(fmt.Sprintf("%s took remaining %s (%s)\n",
					g.RenderPlayerName(playerNum), RenderCash(pot),
					handResult.Name))
				g.PlayerMoney[playerNum] += pot
			}
		}
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(buf.String())}
	if !g.IsFinished() {
		logs = append(logs, g.NewHand()...)
	}
	return logs
}

func (g *Game) CurrentBet() int {
	currentBet := 0
	for _, b := range g.Bets {
		if b > currentBet {
			currentBet = b
		}
	}
	return currentBet
}

func (g *Game) Pot() int {
	total := 0
	for _, b := range g.Bets {
		total += b
	}
	return total
}

func (g *Game) SmallestBet() int {
	bet := 0
	firstRun := true
	for playerNum, _ := range g.ActivePlayers() {
		if g.Bets[playerNum] != 0 && (firstRun || g.Bets[playerNum] < bet) {
			bet = g.Bets[playerNum]
			firstRun = false
		}
	}
	return bet
}

func (g *Game) NewCommunityCards(n int) {
	var cards libcard.Deck
	cards, g.Deck = g.Deck.PopN(n)
	g.CommunityCards = g.CommunityCards.PushMany(cards)
}

func (g *Game) NewBettingRound() {
	if len(g.BettingPlayers()) > 0 {
		g.CurrentPlayer = g.NextBettingPlayerNumFrom(g.CurrentDealer)
	} else {
		g.CurrentPlayer = g.CurrentDealer
	}
	g.FirstBettingPlayer = g.CurrentPlayer
	g.EveryoneHasBetOnce = false
}

func (g *Game) RenderPlayerName(playerNum int) string {
	return render.Player(playerNum)
}

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func (g *Game) PlayerRender(player int) string {
	buf := bytes.NewBufferString("")
	// Table
	buf.WriteString(render.Bold("Community cards:  "))
	buf.WriteString(strings.Join(RenderCards(g.CommunityCards), " "))
	buf.WriteString("\n")
	buf.WriteString(render.Bold("Current pot:      "))
	buf.WriteString(RenderCash(g.Pot()))
	buf.WriteString("\n\n")
	if player >= 0 {
		// Player specific
		buf.WriteString(render.Bold("Your cards:  "))
		buf.WriteString(strings.Join(RenderCards(g.PlayerHands[player]), " "))
		buf.WriteString("\n")
		buf.WriteString(render.Bold("Your cash:   "))
		buf.WriteString(RenderCash(g.PlayerMoney[player]))
		buf.WriteString("\n\n")
	}
	// All players table
	playersTable := [][]render.Cell{
		[]render.Cell{
			render.Cel(render.Bold("Players")),
			render.Cel(render.Bold("Cash")),
			render.Cel(render.Bold("Bet")),
		},
	}
	for tablePlayerNum := 0; tablePlayerNum < g.Players; tablePlayerNum++ {
		name := g.RenderPlayerName(tablePlayerNum)
		if tablePlayerNum == g.CurrentDealer {
			name += " (D)"
		}
		playerRow := []render.Cell{render.Cel(name)}
		if g.PlayerMoney[tablePlayerNum] == 0 && g.Bets[tablePlayerNum] == 0 {
			playerRow = append(playerRow, render.Cel(render.Fg(render.Grey, "Out")))
		} else {
			extraInfo := ""
			if g.FoldedPlayers[tablePlayerNum] {
				extraInfo = render.Fg(render.Grey, "Folded")
			}
			playerRow = append(playerRow,
				render.Cel(RenderCash(g.PlayerMoney[tablePlayerNum])),
				render.Cel(RenderCash(g.Bets[tablePlayerNum])),
				render.Cel(extraInfo),
			)
		}
		playersTable = append(playersTable, playerRow)
	}
	table := render.Table(playersTable, 0, 2)
	buf.WriteString(table)
	return buf.String()
}

func RenderCards(deck libcard.Deck) (output []string) {
	for _, c := range deck {
		output = append(output, render.Bold(c.RenderStandard52FixedWidth()))
	}
	return
}

func (g *Game) CanSeeHand(playerNum, target int) bool {
	return playerNum == target
}

func (g *Game) IsFinished() bool {
	return len(g.RemainingPlayers()) < 2
}

func (g *Game) Winners() []int {
	remainingPlayers := g.RemainingPlayers()
	if len(remainingPlayers) == 1 {
		for _, p := range remainingPlayers {
			return []int{p}
		}
	}
	return []int{}
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) EliminatedPlayerList() (eliminatedPlayers []int) {
	for p := 0; p < g.Players; p++ {
		if g.PlayerMoney[p] == 0 && g.Bets[p] == 0 {
			eliminatedPlayers = append(eliminatedPlayers, p)
		}
	}
	return
}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5, 6, 7, 8, 9}
}

func (g *Game) PlayerState(player int) interface{} {
	panic("unimplemented")
}

func (g *Game) PubState() interface{} {
	panic("unimplemented")
}

func (g *Game) Points() []float32 {
	panic("unimplemented")
}

func (g *Game) Status() brdgme.Status {
	panic("unimplemented")
}

func min(numbers ...int) int {
	l := len(numbers)
	if l == 0 {
		panic("Requires at least one int")
	}
	m := numbers[0]
	if l > 1 {
		for i := 1; i < l; i++ {
			if numbers[i] < m {
				m = numbers[i]
			}
		}
	}
	return m
}

func max(numbers ...int) int {
	l := len(numbers)
	if l == 0 {
		panic("Requires at least one int")
	}
	m := numbers[0]
	if l > 1 {
		for i := 1; i < l; i++ {
			if numbers[i] > m {
				m = numbers[i]
			}
		}
	}
	return m
}
