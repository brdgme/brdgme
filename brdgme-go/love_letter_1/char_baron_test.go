package love_letter_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestCharBaron_Play_win(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.Hands[Mick] = []int{Baron, King}
	g.Hands[Steve] = []int{Prince}
	_, err = g.Command(Mick, "baron steve", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{King}, g.Hands[Mick])
	assert.False(t, g.Eliminated[Mick])
	assert.True(t, g.Eliminated[Steve])
}

func TestCharBaron_Play_tie(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.Hands[Mick] = []int{Baron, Prince}
	g.Hands[Steve] = []int{Prince}
	_, err = g.Command(Mick, "baron steve", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Prince}, g.Hands[Mick])
	assert.False(t, g.Eliminated[Mick])
	assert.False(t, g.Eliminated[Steve])
}

func TestCharBaron_Play_lose(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.Hands[Mick] = []int{Baron, Prince}
	g.Hands[Steve] = []int{King}
	_, err = g.Command(Mick, "baron steve", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{}, g.Hands[Mick])
	assert.True(t, g.Eliminated[Mick])
	assert.False(t, g.Eliminated[Steve])
}

func TestCharBaron_Play_double(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	g.Hands[Mick] = []int{Baron, Baron}
	g.Hands[Steve] = []int{Guard}
	_, err = g.Command(Mick, "baron steve", names)
	assert.NoError(t, err)
	assert.Equal(t, []int{Baron}, g.Hands[Mick])
	assert.False(t, g.Eliminated[Mick])
	assert.True(t, g.Eliminated[Steve])
}
