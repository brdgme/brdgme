package libcard

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	STANDARD_52_SUIT_CLUBS = iota
	STANDARD_52_SUIT_DIAMONDS
	STANDARD_52_SUIT_HEARTS
	STANDARD_52_SUIT_SPADES
	STANDARD_52_SUIT_JOKER
)

const (
	_ = iota // Ignore 0
	STANDARD_52_RANK_ACE
	STANDARD_52_RANK_2
	STANDARD_52_RANK_3
	STANDARD_52_RANK_4
	STANDARD_52_RANK_5
	STANDARD_52_RANK_6
	STANDARD_52_RANK_7
	STANDARD_52_RANK_8
	STANDARD_52_RANK_9
	STANDARD_52_RANK_10
	STANDARD_52_RANK_JACK
	STANDARD_52_RANK_QUEEN
	STANDARD_52_RANK_KING
	STANDARD_52_RANK_ACE_HIGH
)

func Standard52Deck() Deck {
	d := Deck{}
	for suit := STANDARD_52_SUIT_CLUBS; suit <= STANDARD_52_SUIT_SPADES; suit++ {
		for rank := STANDARD_52_RANK_ACE; rank <= STANDARD_52_RANK_KING; rank++ {
			d = append(d, Card{
				Suit: suit,
				Rank: rank,
			})
		}
	}
	return d
}

func Standard52DeckAceHigh() Deck {
	d := Deck{}
	for suit := STANDARD_52_SUIT_CLUBS; suit <= STANDARD_52_SUIT_SPADES; suit++ {
		for rank := STANDARD_52_RANK_2; rank <= STANDARD_52_RANK_ACE_HIGH; rank++ {
			d = append(d, Card{
				Suit: suit,
				Rank: rank,
			})
		}
	}
	return d
}

func Standard52DeckWithJokers() (d Deck) {
	d = Standard52Deck()
	for i := 0; i < 2; i++ {
		d = append(d, Card{
			Suit: STANDARD_52_SUIT_JOKER,
		})
	}
	return d
}

func (c Card) RenderStandard52() string {
	var (
		symbol string
		colour render.Color
		rank   string
	)
	switch c.Suit {
	case STANDARD_52_SUIT_CLUBS:
		symbol = "♣"
		colour = render.Black
	case STANDARD_52_SUIT_DIAMONDS:
		symbol = "♦"
		colour = render.Red
	case STANDARD_52_SUIT_HEARTS:
		symbol = "♥"
		colour = render.Red
	case STANDARD_52_SUIT_SPADES:
		symbol = "♠"
		colour = render.Black
	}
	switch c.Rank {
	case STANDARD_52_RANK_ACE:
		rank = "A"
	case STANDARD_52_RANK_ACE_HIGH:
		rank = "A"
	case STANDARD_52_RANK_JACK:
		rank = "J"
	case STANDARD_52_RANK_QUEEN:
		rank = "Q"
	case STANDARD_52_RANK_KING:
		rank = "K"
	default:
		rank = fmt.Sprintf("%d", c.Rank)
	}
	return render.Fg(colour, fmt.Sprintf("%s%s", symbol, rank))
}

func (c Card) RenderStandard52FixedWidth() string {
	output := c.RenderStandard52()
	if c.Rank != 10 {
		output += " "
	}
	return output
}

func RenderStandard52Hidden() string {
	return render.Fg(render.Grey, "##")
}

func RenderStandard52HiddenFixedWidth() string {
	return RenderStandard52Hidden() + " "
}
