package splendor_1

import (
	"math/rand"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/libcost"
)

const (
	Diamond = iota
	Sapphire
	Emerald
	Ruby
	Onyx
	Gold
	Prestige
)

type Card struct {
	Resource int
	Prestige int
	Cost     libcost.Cost
}

var Resources = []int{
	Diamond,
	Sapphire,
	Emerald,
	Ruby,
	Onyx,
	Gold,
	Prestige,
}

var Gems = []int{
	Diamond,
	Sapphire,
	Emerald,
	Ruby,
	Onyx,
}

func ShuffleCards(cards []Card) []Card {
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	l := len(cards)
	shuffled := make([]Card, l)
	for i, c := range r.Perm(l) {
		shuffled[i] = cards[c]
	}
	return shuffled
}

func Level1Cards() []Card {
	return []Card{
		{
			Diamond,
			0,
			libcost.Cost{
				Sapphire: 1,
				Emerald:  1,
				Ruby:     1,
				Onyx:     1,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Sapphire: 1,
				Emerald:  2,
				Ruby:     1,
				Onyx:     1,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Diamond:  3,
				Sapphire: 1,
				Onyx:     1,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Sapphire: 2,
				Emerald:  2,
				Onyx:     1,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Sapphire: 2,
				Onyx:     2,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Ruby: 2,
				Onyx: 1,
			},
		},
		{
			Diamond,
			1,
			libcost.Cost{
				Emerald: 4,
			},
		},
		{
			Diamond,
			0,
			libcost.Cost{
				Sapphire: 3,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Diamond: 1,
				Emerald: 1,
				Ruby:    1,
				Onyx:    1,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Diamond: 1,
				Emerald: 1,
				Ruby:    2,
				Onyx:    1,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Sapphire: 1,
				Emerald:  3,
				Ruby:     1,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Diamond: 1,
				Emerald: 2,
				Ruby:    2,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Emerald: 2,
				Onyx:    2,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Diamond: 1,
				Onyx:    2,
			},
		},
		{
			Sapphire,
			1,
			libcost.Cost{
				Ruby: 4,
			},
		},
		{
			Sapphire,
			0,
			libcost.Cost{
				Onyx: 3,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 2,
				Emerald:  1,
				Ruby:     1,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 1,
				Emerald:  1,
				Ruby:     1,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Emerald: 1,
				Ruby:    3,
				Onyx:    1,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Diamond:  2,
				Sapphire: 2,
				Ruby:     1,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Diamond: 2,
				Emerald: 2,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Emerald: 2,
				Ruby:    1,
			},
		},
		{
			Onyx,
			0,
			libcost.Cost{
				Emerald: 3,
			},
		},
		{
			Onyx,
			1,
			libcost.Cost{
				Sapphire: 4,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond:  2,
				Sapphire: 1,
				Emerald:  1,
				Onyx:     1,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 1,
				Emerald:  1,
				Onyx:     1,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond: 1,
				Ruby:    1,
				Onyx:    3,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond: 2,
				Ruby:    2,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Sapphire: 2,
				Emerald:  1,
			},
		},
		{
			Ruby,
			1,
			libcost.Cost{
				Diamond: 4,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond: 3,
			},
		},
		{
			Ruby,
			0,
			libcost.Cost{
				Diamond: 2,
				Emerald: 1,
				Onyx:    2,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 1,
				Ruby:     1,
				Onyx:     1,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 1,
				Ruby:     1,
				Onyx:     2,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 3,
				Emerald:  1,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Sapphire: 1,
				Ruby:     2,
				Onyx:     2,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Sapphire: 2,
				Ruby:     2,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Diamond:  2,
				Sapphire: 1,
			},
		},
		{
			Emerald,
			0,
			libcost.Cost{
				Ruby: 3,
			},
		},
		{
			Emerald,
			1,
			libcost.Cost{
				Onyx: 4,
			},
		},
	}
}

func Level2Cards() []Card {
	return []Card{
		{
			Diamond,
			1,
			libcost.Cost{
				Emerald: 3,
				Ruby:    2,
				Onyx:    2,
			},
		},
		{
			Diamond,
			1,
			libcost.Cost{
				Diamond:  2,
				Sapphire: 3,
				Ruby:     3,
			},
		},
		{
			Diamond,
			2,
			libcost.Cost{
				Emerald: 1,
				Ruby:    4,
				Onyx:    2,
			},
		},
		{
			Diamond,
			2,
			libcost.Cost{
				Ruby: 5,
				Onyx: 3,
			},
		},
		{
			Diamond,
			2,
			libcost.Cost{
				Ruby: 5,
			},
		},
		{
			Diamond,
			3,
			libcost.Cost{
				Diamond: 6,
			},
		},
		{
			Sapphire,
			1,
			libcost.Cost{
				Sapphire: 2,
				Emerald:  2,
				Ruby:     3,
			},
		},
		{
			Sapphire,
			1,
			libcost.Cost{
				Sapphire: 2,
				Emerald:  3,
				Onyx:     3,
			},
		},
		{
			Sapphire,
			2,
			libcost.Cost{
				Diamond:  5,
				Sapphire: 3,
			},
		},
		{
			Sapphire,
			2,
			libcost.Cost{
				Diamond: 2,
				Ruby:    1,
				Onyx:    4,
			},
		},
		{
			Sapphire,
			3,
			libcost.Cost{
				Sapphire: 6,
			},
		},
		{
			Sapphire,
			2,
			libcost.Cost{
				Sapphire: 5,
			},
		},
		{
			Onyx,
			1,
			libcost.Cost{
				Diamond: 3,
				Emerald: 3,
				Onyx:    2,
			},
		},
		{
			Onyx,
			2,
			libcost.Cost{
				Sapphire: 1,
				Emerald:  4,
				Ruby:     2,
			},
		},
		{
			Onyx,
			1,
			libcost.Cost{
				Diamond:  3,
				Sapphire: 2,
				Emerald:  2,
			},
		},
		{
			Onyx,
			2,
			libcost.Cost{
				Emerald: 5,
				Ruby:    3,
			},
		},
		{
			Onyx,
			2,
			libcost.Cost{
				Diamond: 5,
			},
		},
		{
			Onyx,
			3,
			libcost.Cost{
				Onyx: 6,
			},
		},
		{
			Ruby,
			1,
			libcost.Cost{
				Diamond: 2,
				Ruby:    2,
				Onyx:    3,
			},
		},
		{
			Ruby,
			1,
			libcost.Cost{
				Sapphire: 3,
				Ruby:     2,
				Onyx:     3,
			},
		},
		{
			Ruby,
			2,
			libcost.Cost{
				Diamond:  1,
				Sapphire: 4,
				Emerald:  2,
			},
		},
		{
			Ruby,
			2,
			libcost.Cost{
				Diamond: 3,
				Onyx:    5,
			},
		},
		{
			Ruby,
			2,
			libcost.Cost{
				Onyx: 5,
			},
		},
		{
			Ruby,
			3,
			libcost.Cost{
				Ruby: 6,
			},
		},
		{
			Emerald,
			2,
			libcost.Cost{
				Emerald: 5,
			},
		},
		{
			Emerald,
			2,
			libcost.Cost{
				Sapphire: 5,
				Emerald:  3,
			},
		},
		{
			Emerald,
			3,
			libcost.Cost{
				Emerald: 6,
			},
		},
		{
			Emerald,
			1,
			libcost.Cost{
				Diamond:  2,
				Sapphire: 3,
				Onyx:     2,
			},
		},
		{
			Emerald,
			1,
			libcost.Cost{
				Diamond: 3,
				Emerald: 2,
				Ruby:    3,
			},
		},
		{
			Emerald,
			2,
			libcost.Cost{
				Diamond:  4,
				Sapphire: 2,
				Onyx:     1,
			},
		},
	}
}

func Level3Cards() []Card {
	return []Card{
		{
			Diamond,
			4,
			libcost.Cost{
				Diamond: 3,
				Ruby:    3,
				Onyx:    6,
			},
		},
		{
			Diamond,
			4,
			libcost.Cost{
				Onyx: 7,
			},
		},
		{
			Diamond,
			5,
			libcost.Cost{
				Diamond: 3,
				Onyx:    7,
			},
		},
		{
			Diamond,
			3,
			libcost.Cost{
				Sapphire: 3,
				Emerald:  3,
				Ruby:     5,
				Onyx:     3,
			},
		},
		{
			Sapphire,
			5,
			libcost.Cost{
				Diamond:  7,
				Sapphire: 3,
			},
		},
		{
			Sapphire,
			4,
			libcost.Cost{
				Diamond:  6,
				Sapphire: 3,
				Onyx:     3,
			},
		},
		{
			Sapphire,
			3,
			libcost.Cost{
				Diamond: 3,
				Emerald: 3,
				Ruby:    3,
				Onyx:    5,
			},
		},
		{
			Sapphire,
			4,
			libcost.Cost{
				Diamond: 7,
			},
		},
		{
			Onyx,
			4,
			libcost.Cost{
				Ruby: 7,
			},
		},
		{
			Onyx,
			4,
			libcost.Cost{
				Emerald: 3,
				Ruby:    6,
				Onyx:    3,
			},
		},
		{
			Onyx,
			5,
			libcost.Cost{
				Ruby: 7,
				Onyx: 3,
			},
		},
		{
			Onyx,
			3,
			libcost.Cost{
				Diamond:  3,
				Sapphire: 3,
				Emerald:  5,
				Ruby:     3,
			},
		},
		{
			Ruby,
			4,
			libcost.Cost{
				Emerald: 7,
			},
		},
		{
			Ruby,
			3,
			libcost.Cost{
				Diamond:  3,
				Sapphire: 5,
				Emerald:  3,
				Onyx:     3,
			},
		},
		{
			Ruby,
			5,
			libcost.Cost{
				Emerald: 7,
				Ruby:    3,
			},
		},
		{
			Ruby,
			4,
			libcost.Cost{
				Sapphire: 3,
				Emerald:  6,
				Ruby:     3,
			},
		},
		{
			Emerald,
			4,
			libcost.Cost{
				Diamond:  3,
				Sapphire: 6,
				Emerald:  3,
			},
		},
		{
			Emerald,
			4,
			libcost.Cost{
				Sapphire: 7,
			},
		},
		{
			Emerald,
			5,
			libcost.Cost{
				Sapphire: 7,
				Emerald:  3,
			},
		},
		{
			Emerald,
			3,
			libcost.Cost{
				Diamond:  5,
				Sapphire: 3,
				Ruby:     3,
				Onyx:     3,
			},
		},
	}
}
