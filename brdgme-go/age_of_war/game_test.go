package age_of_war

import (
	"testing"

	"brdgme-go/assert"
)

func TestGame_New(t *testing.T) {
	g := &Game{}
	_, err := g.New(3)
	assert.NoError(t, err)
}
