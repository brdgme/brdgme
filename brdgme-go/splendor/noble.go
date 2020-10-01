package splendor

import (
	"math/rand"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/libcost"
)

type Noble struct {
	Prestige int
	Cost     libcost.Cost
}

func ShuffleNobles(nobles []Noble) []Noble {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	l := len(nobles)
	shuffled := make([]Noble, l)
	for i, n := range r.Perm(l) {
		shuffled[i] = nobles[n]
	}
	return shuffled
}

func NobleCards() []Noble {
	return []Noble{
		{
			3,
			libcost.Cost{
				Emerald:  3,
				Sapphire: 3,
				Diamond:  3,
			},
		},
		{
			3,
			libcost.Cost{
				Emerald:  3,
				Sapphire: 3,
				Ruby:     3,
			},
		},
		{
			3,
			libcost.Cost{
				Onyx:    3,
				Ruby:    3,
				Diamond: 3,
			},
		},
		{
			3,
			libcost.Cost{
				Onyx:     3,
				Sapphire: 3,
				Diamond:  3,
			},
		},
		{
			3,
			libcost.Cost{
				Onyx:    3,
				Ruby:    3,
				Emerald: 3,
			},
		},
		{
			3,
			libcost.Cost{
				Onyx: 4,
				Ruby: 4,
			},
		},
		{
			3,
			libcost.Cost{
				Onyx:    4,
				Diamond: 4,
			},
		},
		{
			3,
			libcost.Cost{
				Sapphire: 4,
				Diamond:  4,
			},
		},
		{
			3,
			libcost.Cost{
				Sapphire: 4,
				Emerald:  4,
			},
		},
		{
			3,
			libcost.Cost{
				Ruby:    4,
				Emerald: 4,
			},
		},
	}
}
