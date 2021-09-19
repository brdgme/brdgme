package love_letter_1

import "github.com/brdgme/brdgme/brdgme-go/render"

const (
	Princess = 8 - iota
	Countess
	King
	Prince
	Handmaid
	Baron
	Priest
	Guard
)

type Char struct {
	Name   string
	Number int
	Text   string
	Color  render.Color
}

var Cards = map[int]Char{
	Princess: CharPrincess,
	Countess: CharCountess,
	King:     CharKing,
	Prince:   CharPrince,
	Handmaid: CharHandmaid,
	Baron:    CharBaron,
	Priest:   CharPriest,
	Guard:    CharGuard,
}

var Deck = []int{
	Guard,
	Guard,
	Guard,
	Guard,
	Guard,
	Priest,
	Priest,
	Baron,
	Baron,
	Handmaid,
	Handmaid,
	Prince,
	Prince,
	King,
	Countess,
	Princess,
}
