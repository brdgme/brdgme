package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestBuildCityCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceWorkers,
		DiceWorkers,
		DiceFoodOrWorkers,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "take w", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "build 8 city", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 8, g.Boards[Mick].CityProgress)
}

func TestBuildMonumentCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceWorkers,
		DiceWorkers,
		DiceFoodOrWorkers,
		DiceFoodOrWorkers,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "take w w", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "build 10 wall", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 10, g.Boards[Mick].Monuments[MonumentGreatWall])
}

func TestBuildShipCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentShipping] = true
	g.Boards[Mick].Goods[GoodCloth] = 2
	g.Boards[Mick].Goods[GoodWood] = 3
	g.RolledDice = []Die{
		DiceWorkers,
		DiceWorkers,
		DiceFoodOrWorkers,
		DiceFoodOrWorkers,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "take w f", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "build 3 ship", TestPlayers)
	assert.Error(t, err)
	_, err = g.Command(Mick, "build 2 ship", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 2, g.Boards[Mick].Ships)
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodCloth])
	assert.Equal(t, 1, g.Boards[Mick].Goods[GoodWood])
}
