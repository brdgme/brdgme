package sushizock

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

const (
	Mick = iota
	Steve
	BJ
)

var names = []string{"Mick", "Steve", "BJ"}

func TestNew(t *testing.T) {
	g := &Game{}
	if _, err := g.New(2); err != nil {
		t.Fatal(err)
	}
}

func TestRoll(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "roll 1 2 5", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "roll 2 3", names)
	assert.NoError(t, err)
	assert.Equal(t, 0, g.RemainingRolls)
}

func TestTakeBlue(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.RolledDice = []int{DieSushi, DieSushi, DieBlueChopsticks, DieRedChopsticks, DieBones}
	target := g.BlueTiles[1]
	_, err = g.Command(Mick, "take b", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerBlueTiles[Mick])
}

func TestTakeRed(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.RolledDice = []int{DieSushi, DieSushi, DieBlueChopsticks, DieRedChopsticks, DieBones}
	target := g.RedTiles[0]
	_, err = g.Command(Mick, "take r", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerRedTiles[Mick])
}

func TestForceTakeMostNegativeRed(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.RolledDice = []int{DieBones, DieBones, DieBones, DieBones, DieBlueChopsticks}
	g.BlueTiles = Tiles{}
	g.RedTiles = Tiles{
		Tile{Type: TileTypeRed, Value: -2},
		Tile{Type: TileTypeRed, Value: -4},
		Tile{Type: TileTypeRed, Value: -3},
	}
	target := g.RedTiles[1]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerRedTiles[Mick])
}

func TestForceTakeLowestBlue(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.RolledDice = []int{DieSushi, DieSushi, DieSushi, DieSushi, DieBlueChopsticks}
	g.BlueTiles = Tiles{
		Tile{Type: TileTypeBlue, Value: 3},
		Tile{Type: TileTypeBlue, Value: 1},
		Tile{Type: TileTypeBlue, Value: 2},
	}
	g.RedTiles = Tiles{}
	target := g.BlueTiles[1]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerBlueTiles[Mick])
}

func TestStealBlue(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.PlayerBlueTiles[BJ] = Tiles{
		Tile{Type: TileTypeBlue, Value: 3},
		Tile{Type: TileTypeBlue, Value: 1},
		Tile{Type: TileTypeBlue, Value: 2},
	}

	g.RolledDice = []int{DieBlueChopsticks, DieBlueChopsticks, DieBlueChopsticks, DieSushi, DieBones}

	// We should be stealing from the end of the slice
	target := g.PlayerBlueTiles[BJ][2]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "steal bj blue", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerBlueTiles[Mick])
	assert.Equal(t, Tiles{
		Tile{Type: TileTypeBlue, Value: 3},
		Tile{Type: TileTypeBlue, Value: 1},
	}, g.PlayerBlueTiles[BJ])
}

func TestStealRed(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.PlayerRedTiles[Steve] = Tiles{
		Tile{Type: TileTypeRed, Value: -3},
		Tile{Type: TileTypeRed, Value: -1},
		Tile{Type: TileTypeRed, Value: -2},
	}

	g.RolledDice = []int{DieRedChopsticks, DieRedChopsticks, DieRedChopsticks, DieSushi, DieBones}

	// We should be stealing from the end of the slice
	target := g.PlayerRedTiles[Steve][2]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "steal ste r", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerRedTiles[Mick])
	assert.Equal(t, Tiles{
		Tile{Type: TileTypeRed, Value: -3},
		Tile{Type: TileTypeRed, Value: -1},
	}, g.PlayerRedTiles[Steve])
}

func TestStealBlueN(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.PlayerBlueTiles[BJ] = Tiles{
		Tile{Type: TileTypeBlue, Value: 3},
		Tile{Type: TileTypeBlue, Value: 1},
		Tile{Type: TileTypeBlue, Value: 2},
	}

	g.RolledDice = []int{DieBlueChopsticks, DieBlueChopsticks, DieBlueChopsticks, DieBlueChopsticks, DieBones}

	// We should be stealing from the end of the slice
	target := g.PlayerBlueTiles[BJ][0]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "steal bj blue 3", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerBlueTiles[Mick])
	assert.Equal(t, Tiles{
		Tile{Type: TileTypeBlue, Value: 1},
		Tile{Type: TileTypeBlue, Value: 2},
	}, g.PlayerBlueTiles[BJ])
}

func TestStealRedN(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.PlayerRedTiles[Steve] = Tiles{
		Tile{Type: TileTypeRed, Value: -3},
		Tile{Type: TileTypeRed, Value: -1},
		Tile{Type: TileTypeRed, Value: -2},
	}

	g.RolledDice = []int{DieRedChopsticks, DieRedChopsticks, DieRedChopsticks, DieRedChopsticks, DieBones}

	// We should be stealing from the end of the slice
	target := g.PlayerRedTiles[Steve][1]
	_, err = g.Command(Mick, "roll 5", names)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "steal ste r 2", names)
	assert.NoError(t, err)
	assert.Equal(t, Tiles{target}, g.PlayerRedTiles[Mick])
	assert.Equal(t, Tiles{
		Tile{Type: TileTypeRed, Value: -3},
		Tile{Type: TileTypeRed, Value: -2},
	}, g.PlayerRedTiles[Steve])
}
