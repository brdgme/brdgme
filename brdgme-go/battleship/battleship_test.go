package battleship

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

const (
	Mick = iota
	Steve
)

var names = []string{"Mick", "Steve"}

func mockGame(t *testing.T) *Game {
	game := &Game{}
	_, err := game.New(2)
	if err != nil {
		t.Fatal(err)
	}
	return game
}

func TestGame(t *testing.T) {
	g := mockGame(t)
	// Both players place
	if len(g.WhoseTurn()) != 2 {
		t.Fatal("Both players should be placing")
	}
	_, err := g.Command(Mick, "place sub b3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "place car c3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "place des d3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "place cru e3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "place bat f3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Steve, "place sub b3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Steve, "place car c3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Steve, "place des d3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Steve, "place cru e3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Steve, "place bat f3 right", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "shoot b3", names)
	assert.NoError(t, err)
}
