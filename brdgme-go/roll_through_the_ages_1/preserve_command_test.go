package roll_through_the_ages_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestPreserveCommand(t *testing.T) {
	g := &Game{}
	g.NewBlank(3)
	g.Boards[Mick].Developments[DevelopmentPreservation] = true
	g.Boards[Mick].Goods[GoodPottery] = 3
	g.Boards[Mick].Food = 3
	g.Phase = PhasePreserve
	_, err := g.Command(Mick, "preserve", TestPlayers)
	assert.NoError(t, err)
	assert.Equal(t, 6, g.Boards[Mick].Food)
}
