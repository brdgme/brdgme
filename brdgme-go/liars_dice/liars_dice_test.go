package liars_dice

import (
	"testing"
)

func TestStart(t *testing.T) {
	g := &Game{}
	p := []string{"Mick", "Steve", "BJ"}
	if _, err := g.New(len(p)); err != nil {
		t.Fatal(err)
	}
	wt := g.WhoseTurn()
	if len(wt) != 1 || (wt[0] != 0 && wt[0] != 1 && wt[0] != 2) {
		t.Fatal("WhoseTurn doesn't report one of the specified players")
	}
	if len(g.PlayerDice) != len(p) {
		t.Fatal("PlayerDice hasn't been initialised for each player")
	}
	for i := 0; i < g.Players; i++ {
		if len(g.PlayerDice[i]) != START_DICE_COUNT {
			t.Fatalf("PlayerDice for %s has not been initialised to 5 dice",
				p[i])
		}
		for _, d := range g.PlayerDice[i] {
			if d < 1 || d > 6 {
				t.Fatalf("Dice isn't in the range of 1 to 6")
			}
		}
	}
}

func TestExampleRound(t *testing.T) {
	g := &Game{}
	p := []string{"Mick", "Steve", "BJ"}
	if _, err := g.New(len(p)); err != nil {
		t.Fatal(err)
	}
	// Override the game values so we know what's there
	g.PlayerDice = [][]int{
		// Mick
		[]int{1, 3, 4, 4, 6},
		// Steve
		[]int{2, 2, 3, 3, 3},
		// BJ
		[]int{1},
	}
	// First player is mick
	g.CurrentPlayer = 0
	// Make sure we can't call on the first turn
	if _, err := g.Command(0, "call", p); err == nil {
		t.Fatal("Didn't fail when calling on the first turn")
	}
	// Start with a few legit commands
	if _, err := g.Command(0, "bid 2 5", p); err != nil {
		t.Fatal(err)
	}
	if _, err := g.Command(1, "bid 2 6", p); err != nil {
		t.Fatal(err)
	}
	if _, err := g.Command(2, "bid 3 5", p); err != nil {
		t.Fatal(err)
	}
	// Do a few illegal commands and make sure they're picked up
	if _, err := g.Command(0, "bid 3 5", p); err == nil {
		t.Fatal("Didn't fail when making same bid")
	}
	if _, err := g.Command(0, "bid 3 3", p); err == nil {
		t.Fatal("Didn't fail when bidding a lower value dice")
	}
	if _, err := g.Command(0, "bid 2 6", p); err == nil {
		t.Fatal("Didn't fail when reducing the quantity")
	}
	if _, err := g.Command(0, "bid 3 7", p); err == nil {
		t.Fatal("Didn't fail when making an bid of an invalid dice value")
	}
	if _, err := g.Command(3, "bid 6 5", p); err == nil {
		t.Fatal("Didn't fail when BJ barged in")
	}
	// Call it and check
	if _, err := g.Command(0, "call", p); err != nil {
		t.Fatal(err)
	}
	if len(g.PlayerDice[2]) != 0 {
		t.Fatal("BJ should have lost his dice")
	}
	if len(g.PlayerDice[0]) != 5 && len(g.PlayerDice[1]) != 5 {
		t.Fatal("Mick and Steve shouldn't have lost dice")
	}
	if g.CurrentPlayer != 1 {
		t.Fatal("Steve didn't become the current player")
	}
	if len(g.ActivePlayers()) != 2 {
		t.Fatal("BJ wasn't eliminated")
	}
}

func TestPlayerElimination(t *testing.T) {
	g := &Game{}
	p := []string{"Mick", "Steve", "BJ", "Ross"}
	if _, err := g.New(len(p)); err != nil {
		t.Fatal(err)
	}
	g.PlayerDice[0] = []int{}
	g.PlayerDice[2] = []int{}
	eliminated := g.EliminatedPlayerList()
	if len(eliminated) != 2 {
		t.Fatal("Two players weren't eliminated, got:", eliminated)
	}
	if eliminated[0] != 0 {
		t.Fatal("Mick was not eliminated, got:", eliminated)
	}
	if eliminated[1] != 2 {
		t.Fatal("BJ was not eliminated, got:", eliminated)
	}
}
