package texas_holdem_1

import (
	"testing"
)

const (
	MICK = iota
	STEVE
	BJ
)

func mockGame(t *testing.T) *Game {
	g := &Game{}
	_, err := g.New(2)
	if err != nil {
		t.Fatal(err)
	}
	return g
}

func TestStart(t *testing.T) {
	g := mockGame(t)
	if g.Players == 0 {
		t.Fatal("Didn't set players")
	}
}

func TestNextPhaseOnInitialFold(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	if err != nil {
		t.Fatal(err)
	}
	// First player folds
	_, err = g.Command(g.CurrentPlayer, "fold", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 0 {
		t.Fatal("Cards were already drawn:", g.CommunityCards)
	}
	// Next two players call and check, should flop
	_, err = g.Command(g.CurrentPlayer, "call", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 0 {
		t.Fatal("Cards were already drawn:", g.CommunityCards)
	}
	_, err = g.Command(g.CurrentPlayer, "check", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("No flop, community cards:", g.CommunityCards)
	}
}

func TestDealerRaiseWhenLastPlayer(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	if err != nil {
		t.Fatal(err)
	}
	g.CurrentDealer = MICK
	g.CurrentPlayer = MICK
	g.FirstBettingPlayer = MICK
	g.Bets = []int{
		0: 5,
		1: 10,
		2: 0,
	}
	_, err = g.Command(MICK, "call", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 0 {
		t.Fatal("Flopped too early")
	}
	_, err = g.Command(STEVE, "check", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 0 {
		t.Fatal("Flopped too early")
	}
	_, err = g.Command(BJ, "call", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("Flop didn't happen")
	}
	_, err = g.Command(STEVE, "check", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("Turn happened too early")
	}
	_, err = g.Command(BJ, "check", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("Turn happened too early")
	}
	_, err = g.Command(MICK, "raise 10", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("Turn happened too early")
	}
	_, err = g.Command(STEVE, "call", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 {
		t.Fatal("Turn happened too early")
	}
	_, err = g.Command(BJ, "call", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 4 {
		t.Fatal("Turn didn't happen")
	}
}

// https://github.com/Miniand/brdg.me/issues/3
func TestAllInAboveOtherPlayer(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	if err != nil {
		t.Fatal(err)
	}
	g.CommunityCards, g.Deck = g.Deck.PopN(3)
	g.CurrentDealer = 0
	g.CurrentPlayer = 0
	g.FirstBettingPlayer = 0
	g.Bets = []int{
		0: 5,
		1: 10,
	}
	g.PlayerMoney = []int{
		0: 10,
		1: 20,
	}
	// Go all in with BJ
	_, err = g.Command(MICK, "allin", []string{})
	if err != nil {
		t.Fatal(err)
	}
	if len(g.CommunityCards) != 3 || g.CurrentPlayer != 1 {
		t.Fatal("Game progressed without letting Steve call")
	}
}

func TestAllPlayersAllInWhenBlindsBiggerThanCash(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	if err != nil {
		t.Fatal(err)
	}
	g.PlayerMoney = []int{
		0: 3,
		1: 3,
	}
	g.NewHand()
	if !g.IsFinished() {
		t.Fatal("Game didn't finish when players had lower money than blinds")
	}
}

// https://github.com/Miniand/brdg.me/issues/5
func TestNextPlayerIsSkippedOnNextPhaseWhenNoMoney(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	if err != nil {
		t.Fatal(err)
	}
	g.CurrentDealer = 0
	g.CurrentPlayer = 0
	g.FirstBettingPlayer = 0
	g.Bets = []int{
		0: 10,
		1: 6,
		2: 10,
	}
	g.PlayerMoney = []int{
		0: 10,
		1: 0,
		2: 100,
	}
	// Skip to next phase manually
	g.NextPhase()
	if g.CurrentPlayer != 2 {
		t.Fatal("Didn't skip over Mick on new phase even though he is all in, got:", g.CurrentPlayer)
	}
}

func TestEliminatedPlayers(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	if err != nil {
		t.Fatal(err)
	}
	g.PlayerMoney[0] = 0
	g.PlayerMoney[1] = 0
	g.Bets[0] = 0
	g.Bets[1] = 5
	eliminated := g.EliminatedPlayerList()
	if len(eliminated) != 1 || eliminated[0] != MICK {
		t.Fatal("Expected only Mick to be eliminated, got:", eliminated)
	}
}
