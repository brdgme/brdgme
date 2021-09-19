package sushi_go_1

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

func TestGame_Start(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
}

func TestGame_Score_maki(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	score, _ := g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)

	g.Played[Mick] = []int{CardMakiRoll1}
	score, _ = g.Score()
	assert.Equal(t, []int{6, 0, 0}, score)

	g.Played[Steve] = []int{CardMakiRoll1}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 3, 0}, score)

	g.Played[BJ] = []int{CardMakiRoll1}
	score, _ = g.Score()
	assert.Equal(t, []int{2, 2, 2}, score)

	g.Played[Steve] = []int{CardMakiRoll2}
	score, _ = g.Score()
	assert.Equal(t, []int{1, 6, 1}, score)
}

func TestGame_Score_pudding(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.Played[Mick] = []int{CardPudding}
	score, _ := g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)

	g.Round = 3
	score, _ = g.Score()
	assert.Equal(t, []int{6, -3, -3}, score)

	g.Played[BJ] = []int{CardPudding, CardPudding}
	score, _ = g.Score()
	assert.Equal(t, []int{0, -6, 6}, score)

	g.Played[Mick] = []int{CardPudding, CardPudding}
	g.Played[Steve] = []int{CardPudding, CardPudding}
	score, _ = g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)
}

func TestGame_Score_nigiri(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.Played[Mick] = []int{CardEggNigiri}
	score, _ := g.Score()
	assert.Equal(t, []int{1, 0, 0}, score)

	g.Played[Mick] = []int{CardEggNigiri, CardWasabi}
	score, _ = g.Score()
	assert.Equal(t, []int{1, 0, 0}, score)

	g.Played[Mick] = []int{CardWasabi, CardEggNigiri}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 0, 0}, score)

	g.Played[Steve] = []int{CardSalmonNigiri}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 2, 0}, score)

	g.Played[Steve] = []int{CardWasabi, CardSalmonNigiri}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 6, 0}, score)

	g.Played[BJ] = []int{CardSquidNigiri}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 6, 3}, score)

	g.Played[BJ] = []int{CardWasabi, CardSquidNigiri}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 6, 9}, score)
}

func TestGame_Score_tempura(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.Played[Mick] = []int{CardTempura}
	score, _ := g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)

	g.Played[Mick] = []int{CardTempura, CardTempura}
	score, _ = g.Score()
	assert.Equal(t, []int{5, 0, 0}, score)

	g.Played[Mick] = []int{CardTempura, CardTempura, CardTempura}
	score, _ = g.Score()
	assert.Equal(t, []int{5, 0, 0}, score)

	g.Played[Mick] = []int{CardTempura, CardTempura, CardTempura, CardTempura}
	score, _ = g.Score()
	assert.Equal(t, []int{10, 0, 0}, score)
}

func TestGame_Score_sashimi(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.Played[Mick] = []int{CardSashimi}
	score, _ := g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)

	g.Played[Mick] = []int{CardSashimi, CardSashimi}
	score, _ = g.Score()
	assert.Equal(t, []int{0, 0, 0}, score)

	g.Played[Mick] = []int{CardSashimi, CardSashimi, CardSashimi}
	score, _ = g.Score()
	assert.Equal(t, []int{10, 0, 0}, score)

	g.Played[Mick] = []int{CardSashimi, CardSashimi, CardSashimi, CardSashimi}
	score, _ = g.Score()
	assert.Equal(t, []int{10, 0, 0}, score)
}

func TestGame_Score_dumpling(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	g.Played[Mick] = []int{
		CardDumpling,
	}
	score, _ := g.Score()
	assert.Equal(t, []int{1, 0, 0}, score)

	g.Played[Mick] = []int{
		CardDumpling,
		CardDumpling,
	}
	score, _ = g.Score()
	assert.Equal(t, []int{3, 0, 0}, score)

	g.Played[Mick] = []int{
		CardDumpling,
		CardDumpling,
		CardDumpling,
	}
	score, _ = g.Score()
	assert.Equal(t, []int{6, 0, 0}, score)

	g.Played[Mick] = []int{
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
	}
	score, _ = g.Score()
	assert.Equal(t, []int{10, 0, 0}, score)

	g.Played[Mick] = []int{
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
	}
	score, _ = g.Score()
	assert.Equal(t, []int{15, 0, 0}, score)

	g.Played[Mick] = []int{
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
		CardDumpling,
	}
	score, _ = g.Score()
	assert.Equal(t, []int{15, 0, 0}, score)
}
