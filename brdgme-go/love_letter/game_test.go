package love_letter

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

func TestGame_IsFinished(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[2] - 1
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[2]
	assert.True(t, g.IsFinished())

	g = &Game{}
	_, err = g.New(3)
	assert.NoError(t, err)
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[3] - 1
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[3]
	assert.True(t, g.IsFinished())

	g = &Game{}
	_, err = g.New(4)
	assert.NoError(t, err)
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[4] - 1
	assert.False(t, g.IsFinished())
	g.PlayerPoints[Mick] = endScores[4]
	assert.True(t, g.IsFinished())
}
