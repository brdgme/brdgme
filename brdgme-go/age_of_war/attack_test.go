package age_of_war

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGame_Attack(t *testing.T) {
	g := Game{}
	_, err := g.New(2)
	assert.NoError(t, err)
	_, err = g.Command(g.CurrentPlayer, "attack azu", []string{})
	assert.NoError(t, err)
}
