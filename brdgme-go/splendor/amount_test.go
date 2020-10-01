package splendor

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
	"github.com/brdgme/brdgme/brdgme-go/libcost"
)

func TestCanAfford(t *testing.T) {
	assert.True(t, CanAfford(libcost.Cost{
		Emerald: 2,
		Gold:    1,
	}, libcost.Cost{
		Emerald: 3,
	}))
	assert.False(t, CanAfford(libcost.Cost{
		Emerald: 2,
		Gold:    1,
	}, libcost.Cost{
		Emerald: 4,
	}))
}
