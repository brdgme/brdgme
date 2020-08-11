package libcard

type Card struct {
	Suit int
	Rank int
}

// Sort by suit first, then card
func (c Card) Compare(otherC Card) int {
	if c.Suit < otherC.Suit {
		return -1
	} else if c.Suit > otherC.Suit {
		return 1
	} else if c.Rank < otherC.Rank {
		return -1
	} else if c.Rank > otherC.Rank {
		return 1
	}
	return 0
}
