package sushi_go

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestPlayCommand_Call(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
	assert.Equal(t, []int{Mick, Steve, BJ}, g.WhoseTurn())

	// Mick plays a card
	mickCard := g.Hands[Mick][0]
	_, err = g.Command(Mick, "play 1", names)
	assert.NoError(t, err)
	assert.Equal(t, CardPlayed, g.Hands[Mick][0])
	assert.Equal(t, []int{mickCard}, g.Playing[Mick])
	assert.Equal(t, []int{Steve, BJ}, g.WhoseTurn())

	// BJ plays a card
	bjCard := g.Hands[BJ][1]
	_, err = g.Command(BJ, "play 2", names)
	assert.NoError(t, err)
	assert.Equal(t, CardPlayed, g.Hands[BJ][1])
	assert.Equal(t, []int{bjCard}, g.Playing[BJ])
	assert.Equal(t, []int{Steve}, g.WhoseTurn())

	// Steve plays a card
	steveHandLen := len(g.Hands[Steve])
	steveCard := g.Hands[Steve][8]
	_, err = g.Command(Steve, "play 9", names)
	assert.NoError(t, err)
	assert.Len(t, g.Hands[Steve], steveHandLen-1)
	// Plays should have happened now.
	assert.Equal(t, []int{mickCard}, g.Played[Mick])
	assert.Equal(t, []int{bjCard}, g.Played[BJ])
	assert.Equal(t, []int{steveCard}, g.Played[Steve])
	assert.Equal(t, []int{Mick, Steve, BJ}, g.WhoseTurn())
}

func TestPlayCommand_Call_chopsticks(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)

	// Prepare hands
	g.Hands[Mick] = []int{
		CardDumpling,
		CardMakiRoll3,
		CardMakiRoll2,
		CardMakiRoll1,
	}
	g.Hands[Steve] = []int{
		CardDumpling,
		CardSalmonNigiri,
		CardSquidNigiri,
		CardEggNigiri,
	}
	g.Played[Mick] = []int{
		CardSquidNigiri,
		CardEggNigiri,
		CardDumpling,
	}
	g.Played[Steve] = []int{
		CardPudding,
		CardChopsticks,
		CardSashimi,
	}

	// Mick tries to play two cards but can't without chopsticks
	_, err = g.Command(Mick, "play 1 2", names)
	assert.Error(t, err)
	// It should work after giving Mick chopsticks
	g.Played[Mick][1] = CardChopsticks
	_, err = g.Command(Mick, "play 1 2", names)
	assert.NoError(t, err)

	// Play with the rest
	_, err = g.Command(Steve, "play 3", names)
	assert.NoError(t, err)
	_, err = g.Command(BJ, "play 2", names)
	assert.NoError(t, err)

	// Make sure Mick got his hand from Steve
	assert.Equal(t, []int{
		CardDumpling,
		CardSalmonNigiri,
		CardEggNigiri,
	}, g.Hands[Mick])
	// Make sure BJ got his hand from Mick
	assert.Equal(t, []int{
		CardMakiRoll2,
		CardMakiRoll1,
		CardChopsticks,
	}, g.Hands[BJ])
}

func TestPlayCommand_Call_dummyPlayTwo(t *testing.T) {
	g := &Game{}
	_, err := g.New(2)
	assert.NoError(t, err)

	// Mick isn't allowed to play both cards if the dummy card hasn't had one
	// yet
	g.Played[Mick] = []int{CardChopsticks}
	g.Hands[Mick] = []int{CardMakiRoll1, CardMakiRoll2}
	_, err = g.Command(Mick, "play 1 2", names)
	assert.Error(t, err)

	// Should be fine if dummy has already had a card played.
	g.Playing[Dummy] = []int{CardMakiRoll1}
	_, err = g.Command(Mick, "play 1 2", names)
	assert.NoError(t, err)
}
