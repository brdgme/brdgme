package roll_through_the_ages

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestBuyCommandCoins(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceCoins,
		DiceCoins,
	}
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "buy leader", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, true, g.Boards[Mick].Developments[DevelopmentLeadership])
	assert.NotEqual(t, PhaseBuy, g.Phase)
}

func TestBuyCommandCoinsWithCoinage(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceCoins,
		DiceCoins,
	}
	g.Boards[Mick].Developments[DevelopmentCoinage] = true
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "buy cara", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, true, g.Boards[Mick].Developments[DevelopmentCaravans])
	assert.NotEqual(t, PhaseBuy, g.Phase)
}

func TestBuyCommandGoodsSpecific(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
	}
	g.Boards[Mick].Developments[DevelopmentCoinage] = true
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "buy agri wood stone pot cloth spear", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, true, g.Boards[Mick].Developments[DevelopmentAgriculture])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodWood])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodStone])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodPottery])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodCloth])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodSpearhead])
	assert.NotEqual(t, PhaseBuy, g.Phase)
}

func TestBuyCommandGoodsAll(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.RolledDice = []Die{
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
		DiceGood,
	}
	g.Boards[Mick].Developments[DevelopmentCoinage] = true
	_, err := g.Command(Mick, "next", TestPlayers)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "buy agri all", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, true, g.Boards[Mick].Developments[DevelopmentAgriculture])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodWood])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodStone])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodPottery])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodCloth])
	assert.Equal(t, 0, g.Boards[Mick].Goods[GoodSpearhead])
	assert.NotEqual(t, PhaseBuy, g.Phase)
}
