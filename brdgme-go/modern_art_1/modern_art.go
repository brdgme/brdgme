package modern_art_1

import (
	"bytes"
	"errors"
	"fmt"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcard"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	INITIAL_MONEY = 100
)

type State int

const (
	STATE_PLAY_CARD State = iota
	STATE_AUCTION
)
const (
	SUIT_LITE_METAL = iota
	SUIT_YOKO
	SUIT_CHRISTINE_P
	SUIT_KARL_GITTER
	SUIT_KRYPTO
)
const (
	RANK_OPEN = iota
	RANK_FIXED_PRICE
	RANK_SEALED
	RANK_DOUBLE
	RANK_ONCE_AROUND
)

var roundCards = map[int]map[int]int{
	3: {
		0: 10,
		1: 6,
		2: 6,
		3: 0,
	},
	4: {
		0: 9,
		1: 4,
		2: 4,
		3: 0,
	},
	5: {
		0: 8,
		1: 3,
		2: 3,
		3: 0,
	},
}

var suits = []int{
	SUIT_LITE_METAL,
	SUIT_YOKO,
	SUIT_CHRISTINE_P,
	SUIT_KARL_GITTER,
	SUIT_KRYPTO,
}

var suitNames = map[int]string{
	SUIT_LITE_METAL:  "Lite Metal",
	SUIT_YOKO:        "Yoko",
	SUIT_CHRISTINE_P: "Christine P",
	SUIT_KARL_GITTER: "Karl Gitter",
	SUIT_KRYPTO:      "Krypto",
}

var suitCodes = map[int]string{
	SUIT_LITE_METAL:  "lm",
	SUIT_YOKO:        "yo",
	SUIT_CHRISTINE_P: "cp",
	SUIT_KARL_GITTER: "kg",
	SUIT_KRYPTO:      "kr",
}

var suitColours = map[int]render.Color{
	SUIT_LITE_METAL:  render.Yellow,
	SUIT_YOKO:        render.Green,
	SUIT_CHRISTINE_P: render.Red,
	SUIT_KARL_GITTER: render.Blue,
	SUIT_KRYPTO:      render.Brown,
}

var ranks = []int{
	RANK_OPEN,
	RANK_FIXED_PRICE,
	RANK_SEALED,
	RANK_DOUBLE,
	RANK_ONCE_AROUND,
}

var rankNames = map[int]string{
	RANK_OPEN:        "Open",
	RANK_FIXED_PRICE: "Fixed Price",
	RANK_SEALED:      "Sealed",
	RANK_DOUBLE:      "Double",
	RANK_ONCE_AROUND: "Once Around",
}

var rankCodes = map[int]string{
	RANK_OPEN:        "op",
	RANK_FIXED_PRICE: "fp",
	RANK_SEALED:      "sl",
	RANK_DOUBLE:      "db",
	RANK_ONCE_AROUND: "oa",
}

var cardDistribution = map[int]map[int]int{
	SUIT_LITE_METAL: {
		RANK_OPEN:        3,
		RANK_FIXED_PRICE: 2,
		RANK_SEALED:      2,
		RANK_DOUBLE:      2,
		RANK_ONCE_AROUND: 3,
	},
	SUIT_YOKO: {
		RANK_OPEN:        3,
		RANK_FIXED_PRICE: 3,
		RANK_SEALED:      3,
		RANK_DOUBLE:      2,
		RANK_ONCE_AROUND: 2,
	},
	SUIT_CHRISTINE_P: {
		RANK_OPEN:        3,
		RANK_FIXED_PRICE: 3,
		RANK_SEALED:      3,
		RANK_DOUBLE:      2,
		RANK_ONCE_AROUND: 3,
	},
	SUIT_KARL_GITTER: {
		RANK_OPEN:        3,
		RANK_FIXED_PRICE: 3,
		RANK_SEALED:      3,
		RANK_DOUBLE:      3,
		RANK_ONCE_AROUND: 3,
	},
	SUIT_KRYPTO: {
		RANK_OPEN:        4,
		RANK_FIXED_PRICE: 3,
		RANK_SEALED:      3,
		RANK_DOUBLE:      3,
		RANK_ONCE_AROUND: 3,
	},
}

var _ brdgme.Gamer = &Game{}

type Game struct {
	Players             int
	PlayerMoney         map[int]int
	PlayerHands         map[int]libcard.Deck
	PlayerPurchases     map[int]libcard.Deck
	State               State
	Round               int
	Deck                libcard.Deck
	CurrentPlayer       int
	ValueBoard          []map[int]int
	Finished            bool
	CurrentlyAuctioning libcard.Deck
	Bids                map[int]int
}

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

func (g *Game) Placings() []int {
	metrics := [][]int{}
	for p := 0; p < g.Players; p++ {
		metrics = append(metrics, []int{
			g.PlayerMoney[p],
		})
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) Points() []float32 {
	points := make([]float32, g.Players)
	if !g.IsFinished() {
		// "Points" are cash, and cash is secret until the end of the game
		return points
	}

	for p := 0; p < g.Players; p++ {
		points[p] = float32(g.PlayerMoney[p])
	}
	return points
}

func (g *Game) PlayerRender(pNum int) string {
	output := bytes.Buffer{}
	// Auction specific
	if g.IsAuction() {
		output.WriteString(fmt.Sprintf(
			"%s is auctioning %s\n\n",
			render.Player(g.CurrentPlayer),
			RenderCardNames(g.CurrentlyAuctioning),
		))
		if g.AuctionType() != RANK_SEALED {
			bidder, bid := g.HighestBidder()
			output.WriteString(fmt.Sprintf(
				"%s %s by %s\n",
				render.Bold("Current bid:"),
				RenderMoney(bid),
				render.Player(bidder),
			))
		}
		output.WriteString("\n")
	}
	if pNum >= 0 {
		// Player money
		output.WriteString(fmt.Sprintf(
			"%s %s\n\n",
			render.Bold("Your money:"),
			RenderMoney(g.PlayerMoney[pNum]),
		))
		// Player cards
		output.WriteString(render.Bold("Your cards:\n"))
		for _, c := range g.PlayerHands[pNum] {
			output.WriteString(fmt.Sprintf("%s\n",
				RenderCardNameCode(c)))
		}
		output.WriteString("\n")
	}
	// Players
	cells := [][]render.Cell{
		{
			render.Cel(render.Bold("Players")),
			render.Cel(render.Bold("Purchases")),
		},
	}
	for opNum := 0; opNum < g.Players; opNum++ {
		cards := []string{}
		if len(g.PlayerPurchases[opNum]) > 0 {
			for _, c := range g.PlayerPurchases[opNum] {
				src := c
				cards = append(cards, RenderCardCode(src))
			}
		} else {
			cards = append(cards, render.Fg(render.Grey, "None"))
		}
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(opNum)),
			render.Cel(strings.Join(cards, " ")),
		})
	}
	table := render.Table(cells, 0, 2)
	output.WriteString(table)
	output.WriteString("\n\n")
	// Artists
	cells = [][]render.Cell{
		{
			render.Cel(render.Bold("Artist")),
			render.Cel(render.Bold("R1")),
			render.Cel(render.Bold("R2")),
			render.Cel(render.Bold("R3")),
			render.Cel(render.Bold("R4")),
			render.Cel(render.Bold("Total")),
		},
	}
	for _, s := range suits {
		row := []render.Cell{
			render.Cel(RenderSuit(s)),
		}
		for i := 0; i < 4; i++ {
			if len(g.ValueBoard) > i {
				row = append(row, render.Cel(RenderMoney(g.ValueBoard[i][s])))
			} else {
				row = append(row, render.Cel("."))
			}
		}
		row = append(row, render.Cel(RenderMoney(g.SuitValue(s))))
		cells = append(cells, row)
	}
	table = render.Table(cells, 0, 2)
	output.WriteString(table)
	return output.String()
}

func (g *Game) PubRender() string {
	return g.PlayerRender(-1)
}

func (g *Game) SuitCardsOnTable(suit int) int {
	count := 0
	for pNum := 0; pNum < g.Players; pNum++ {
		for _, c := range g.PlayerPurchases[pNum] {
			if c.Suit == suit {
				count += 1
			}
		}
	}
	for _, c := range g.CurrentlyAuctioning {
		if c.Suit == suit {
			count += 1
		}
	}
	return count
}

func (g *Game) SuitValue(suit int) int {
	value := 0
	for _, values := range g.ValueBoard {
		value += values[suit]
	}
	return value
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 3 || players > 5 {
		return nil, errors.New("Modern Art requires between 3 and 5 players")
	}
	g.Players = players
	g.PlayerMoney = map[int]int{}
	g.PlayerHands = map[int]libcard.Deck{}
	for i := 0; i < g.Players; i++ {
		g.PlayerMoney[i] = INITIAL_MONEY
		g.PlayerHands[i] = libcard.Deck{}
	}
	g.ValueBoard = []map[int]int{}
	g.CurrentlyAuctioning = libcard.Deck{}
	g.Deck = Deck().Shuffle()
	return g.StartRound(), nil
}

func (g *Game) StartRound() []brdgme.Log {
	logs := []brdgme.Log{}
	g.State = STATE_PLAY_CARD
	numCards := roundCards[g.Players][g.Round]
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf("Start of round %d", g.Round+1)))
	if numCards > 0 {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Dealing %d cards to each player", numCards)))
	}
	for i := 0; i < g.Players; i++ {
		g.PlayerPurchases = map[int]libcard.Deck{}
		if numCards > 0 {
			cards, remaining := g.Deck.PopN(numCards)
			g.PlayerHands[i] = g.PlayerHands[i].PushMany(cards).Sort()
			g.Deck = remaining
		}
	}
	return logs
}

func (g *Game) EndRound() []brdgme.Log {
	logs := []brdgme.Log{brdgme.NewPublicLog(
		render.Bold("It is the end of the round"),
	)}
	// Add values to artists
	g.CurrentlyAuctioning = libcard.Deck{}
	values := map[int]int{}
	scored := map[int]bool{}
	counts := map[int]int{}
	for _, s := range suits {
		counts[s] = g.SuitCardsOnTable(s)
	}
	for _, v := range []int{30, 20, 10} {
		highest := -1
		highestCount := -1
		for _, s := range suits {
			if !scored[s] && counts[s] > highestCount {
				highest = s
				highestCount = counts[s]
			}
		}
		scored[highest] = true
		values[highest] = v
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Adding %s to the value of %s (%d cards)",
			RenderMoney(v), RenderSuit(highest), highestCount)))
	}
	g.ValueBoard = append(g.ValueBoard, values)
	// Pay out purchased cards
	for pNum := 0; pNum < g.Players; pNum++ {
		pTotal := 0
		for _, c := range g.PlayerPurchases[pNum] {
			pTotal += g.SuitValue(c.Suit)
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Paying %s %s for selling all their cards",
			render.Player(pNum), RenderMoney(pTotal))))
		g.PlayerMoney[pNum] += pTotal
	}
	if g.Round == 3 {
		moneyTable := [][]render.Cell{}
		for pNum := 0; pNum < g.Players; pNum++ {
			moneyTable = append(moneyTable, []render.Cell{
				render.Cel(render.Player(pNum)),
				render.Cel(RenderMoney(g.PlayerMoney[pNum])),
			})
		}
		table := render.Table(moneyTable, 0, 1)
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s\n%s",
			render.Bold("End of the game, final player money:"),
			table,
		)))
		g.Finished = true
	} else {
		g.Round += 1
		g.NextPlayer()
		logs = append(logs, g.StartRound()...)
	}
	return logs
}

func (g *Game) IsFinished() bool {
	return g.Finished
}

func (g *Game) WhoseTurn() []int {
	if g.IsFinished() {
		return []int{}
	}
	switch g.State {
	case STATE_PLAY_CARD:
		return []int{g.CurrentPlayer}
	case STATE_AUCTION:
		switch g.AuctionType() {
		case RANK_OPEN:
			players := []int{}
			highestBidder, _ := g.HighestBidder()
			for pNum := 0; pNum < g.Players; pNum++ {
				if bid, ok := g.Bids[pNum]; pNum != highestBidder &&
					(!ok || bid > 0) {
					players = append(players, pNum)
				}
			}
			return players
		case RANK_FIXED_PRICE, RANK_DOUBLE:
			// Find the first person without a bid including current player
			for i := 0; i < g.Players; i++ {
				p := (i + g.CurrentPlayer) % g.Players
				if _, ok := g.Bids[p]; !ok {
					return []int{p}
				}
			}
		case RANK_ONCE_AROUND:
			// Find the first person without a bid after current player
			highestBid := 0
			for i := 0; i < g.Players; i++ {
				p := (1 + i + g.CurrentPlayer) % g.Players
				bid, ok := g.Bids[p]
				if ok && bid > highestBid {
					highestBid = bid
				}
				if !ok && !(highestBid == 0 && p == g.CurrentPlayer) {
					return []int{p}
				}
			}
		case RANK_SEALED:
			players := []int{}
			for pNum := 0; pNum < g.Players; pNum++ {
				if _, ok := g.Bids[pNum]; !ok {
					players = append(players, pNum)
				}
			}
			return players
		}
	}
	return []int{}
}

func (g *Game) HighestBidder() (player, bid int) {
	bid = -1
	for i := g.CurrentPlayer; i < g.CurrentPlayer+g.Players; i++ {
		p := i % g.Players
		if g.Bids[p] > bid {
			player = p
			bid = g.Bids[p]
		}
	}
	return
}

func (g *Game) NextPlayer() {
	g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players
}

func (g *Game) CanPlay(player int) bool {
	return !g.IsFinished() && g.IsPlayersTurn(player) &&
		g.State == STATE_PLAY_CARD
}

func (g *Game) CanPass(player int) bool {
	if g.IsAuction() {
		switch g.AuctionType() {
		case RANK_OPEN, RANK_SEALED, RANK_DOUBLE, RANK_ONCE_AROUND:
			return g.IsPlayersTurn(player)
		case RANK_FIXED_PRICE:
			return player != g.CurrentPlayer &&
				g.IsPlayersTurn(player)
		}
	}
	return false
}

func (g *Game) CanBid(player int) bool {
	if g.IsAuction() {
		switch g.AuctionType() {
		case RANK_OPEN, RANK_SEALED, RANK_ONCE_AROUND:
			return g.IsPlayersTurn(player)
		}
	}
	return false
}

func (g *Game) CanAdd(player int) bool {
	return g.IsAuction() && g.AuctionType() == RANK_DOUBLE &&
		g.IsPlayersTurn(player) && len(g.PlayerHands[player]) > 0
}

func (g *Game) CanBuy(player int) bool {
	return g.IsAuction() && g.AuctionType() == RANK_FIXED_PRICE &&
		g.IsPlayersTurn(player) && g.CurrentPlayer != player
}

func (g *Game) CanSetPrice(player int) bool {
	return g.IsAuction() && g.AuctionType() == RANK_FIXED_PRICE &&
		g.IsPlayersTurn(player) && g.CurrentPlayer == player
}

func (g *Game) IsAuction() bool {
	return g.State == STATE_AUCTION
}

func (g *Game) AuctionType() int {
	return g.AuctionCard().Rank
}

func (g *Game) AuctionCard() libcard.Card {
	if len(g.CurrentlyAuctioning) == 0 {
		return libcard.Card{}
	}
	return g.CurrentlyAuctioning[len(g.CurrentlyAuctioning)-1]
}

func RenderCardNameCode(c libcard.Card) string {
	return RenderInSuit(c.Suit, fmt.Sprintf("(%s) %s",
		CardCode(c), CardName(c)))
}

func RenderCardName(c libcard.Card) string {
	return RenderInSuit(c.Suit, CardName(c))
}

func RenderCardCode(c libcard.Card) string {
	return RenderInSuit(c.Suit, CardCode(c))
}

func RenderSuit(suit int) string {
	return RenderInSuit(suit, suitNames[suit])
}

func RenderInSuit(suit int, s string) string {
	return render.Markup(s, suitColours[suit], true)
}

func CardName(c libcard.Card) string {
	return fmt.Sprintf("%s - %s", suitNames[c.Suit], rankNames[c.Rank])
}

func CardCode(c libcard.Card) string {
	return fmt.Sprintf("%s%s", suitCodes[c.Suit], rankCodes[c.Rank])
}

func (g *Game) SetPrice(player, price int) ([]brdgme.Log, error) {
	if !g.CanSetPrice(player) {
		return nil, errors.New("You're not able to set the price at the moment")
	}
	if price <= 0 {
		return nil, errors.New("The price you set must be higher than 0")
	}
	if price > g.PlayerMoney[player] {
		return nil, errors.New("You can't set the price higher than your current money")
	}
	g.Bids[player] = price
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s set the price to %s",
		render.Player(player),
		RenderMoney(price),
	))}, nil
}

func (g *Game) Buy(player int) ([]brdgme.Log, error) {
	if !g.CanBuy(player) {
		return nil, errors.New("You're not able to buy the card at the moment")
	}
	price := g.Bids[g.CurrentPlayer]
	if price > g.PlayerMoney[player] {
		return nil, errors.New("You don't have enough money to buy the card")
	}
	return g.SettleAuction(player, price), nil
}

func (g *Game) PlayCard(player int, c libcard.Card) ([]brdgme.Log, error) {
	if !g.CanPlay(player) {
		return nil, errors.New("You're not able to play a card at the moment")
	}
	g.CurrentlyAuctioning = libcard.Deck{}
	return g.AddCardToAuction(player, c)
}

func RenderMoney(amount int) string {
	return render.Markup(fmt.Sprintf("$%d", amount), render.Green, true)
}

func (g *Game) AddCardToAuction(player int, c libcard.Card) ([]brdgme.Log, error) {
	remaining, removed := g.PlayerHands[player].Remove(c, 1)
	if removed != 1 {
		return nil, errors.New("You do not have that card in your hand")
	}
	g.CurrentPlayer = player
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s",
		render.Player(player),
		RenderCardName(c),
	))}
	g.PlayerHands[player] = remaining
	g.CurrentlyAuctioning = g.CurrentlyAuctioning.Push(c)
	g.Bids = map[int]int{}
	g.State = STATE_AUCTION
	if g.SuitCardsOnTable(c.Suit) >= 5 {
		logs = append(logs, g.EndRound()...)
	}
	return logs, nil
}

func RenderCardNames(d libcard.Deck) string {
	cardStrings := []string{}
	for _, c := range d {
		cardStrings = append(cardStrings,
			RenderCardName(c))
	}
	return strings.Join(cardStrings, " and ")
}

func (g *Game) SettleAuction(winner, price int) []brdgme.Log {
	logs := []brdgme.Log{}
	g.PlayerMoney[winner] -= price
	g.PlayerPurchases[winner] = g.PlayerPurchases[winner].
		PushMany(g.CurrentlyAuctioning).Sort()
	paidTo := "the bank"
	if winner != g.CurrentPlayer {
		g.PlayerMoney[g.CurrentPlayer] += price
		paidTo = render.Player(g.CurrentPlayer)
	}
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s bought %s, paying %s to %s",
		render.Player(winner),
		RenderCardNames(g.CurrentlyAuctioning),
		RenderMoney(price),
		paidTo,
	)))

	g.State = STATE_PLAY_CARD
	g.NextPlayer()
	for len(g.PlayerHands[g.CurrentPlayer]) == 0 {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"Skipping %s as they have no cards",
			render.Player(g.CurrentPlayer),
		)))
		g.NextPlayer()
	}

	return logs
}

func (g *Game) Pass(player int) ([]brdgme.Log, error) {
	if !g.CanPass(player) {
		return nil, errors.New("You're not able to pass at the moment")
	}
	logs := []brdgme.Log{}
	g.Bids[player] = 0
	if g.AuctionType() != RANK_SEALED {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s passed",
			render.Player(player),
		)))
	}
	switch g.AuctionType() {
	case RANK_FIXED_PRICE:
		if len(g.Bids) == g.Players {
			logs = append(logs, g.SettleAuction(g.CurrentPlayer, g.Bids[g.CurrentPlayer])...)
		}
	default:
		if len(g.WhoseTurn()) == 0 {
			logs = append(logs, g.SettleAuction(g.HighestBidder())...)
		}
	}
	return logs, nil
}

func (g *Game) Bid(player, amount int) ([]brdgme.Log, error) {
	if !g.CanBid(player) {
		return nil, errors.New("You're not able to bid at the moment")
	}
	if amount > g.PlayerMoney[player] {
		return nil, fmt.Errorf(
			"You must not bid higher than the money you have, which is $%d",
			g.PlayerMoney[player])
	}
	if g.AuctionType() != RANK_SEALED {
		_, highestBid := g.HighestBidder()
		if amount <= highestBid {
			return nil, fmt.Errorf("You must bid higher than $%d", highestBid)
		}
	}
	g.Bids[player] = amount
	logs := []brdgme.Log{}
	if g.AuctionType() != RANK_SEALED {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s bid %s",
			render.Player(player),
			RenderMoney(amount),
		)))
	}
	if len(g.WhoseTurn()) == 0 {
		logs = append(logs, g.SettleAuction(g.HighestBidder())...)
	}
	return logs, nil
}

func (g *Game) AddCard(player int, c libcard.Card) ([]brdgme.Log, error) {
	if !g.CanAdd(player) {
		return nil, errors.New("You're not able to add a card at the moment")
	}
	if g.AuctionCard().Suit != c.Suit {
		return nil, errors.New("The artist of the card must match the existing one")
	}
	if c.Rank == RANK_DOUBLE {
		return nil, errors.New("You are not allowed to add a second double auction")
	}
	return g.AddCardToAuction(player, c)
}

func (g *Game) IsPlayersTurn(player int) bool {
	for _, p := range g.WhoseTurn() {
		if p == player {
			return true
		}
	}
	return false
}

func Deck() libcard.Deck {
	d := libcard.Deck{}
	for suit, suitCards := range cardDistribution {
		for rank, n := range suitCards {
			for i := 0; i < n; i++ {
				d = d.Push(libcard.Card{Suit: suit, Rank: rank})
			}
		}
	}
	return d
}

func ParseCard(s string) (libcard.Card, error) {
	raw := strings.ToUpper(strings.TrimSpace(s))
	c := libcard.Card{}
	found := false
	for code, prefix := range suitCodes {
		upperPrefix := strings.ToUpper(prefix)
		if strings.HasPrefix(raw, upperPrefix) {
			found = true
			c.Suit = code
			raw = strings.TrimPrefix(raw, upperPrefix)
			break
		}
	}
	if !found {
		return c, errors.New("Could not find the artist in card code")
	}
	for code, suffix := range rankCodes {
		upperSuffix := strings.ToUpper(suffix)
		if strings.HasSuffix(raw, upperSuffix) {
			found = true
			c.Rank = code
			raw = strings.TrimSuffix(raw, upperSuffix)
			break
		}
	}
	if !found {
		return c, errors.New("Could not find the auction type in card code")
	}
	return c, nil
}
