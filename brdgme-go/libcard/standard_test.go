package libcard

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

func TestRenderStandard52(t *testing.T) {
	c := Card{
		Suit: STANDARD_52_SUIT_CLUBS,
		Rank: STANDARD_52_RANK_ACE,
	}
	expected := render.Fg(render.Black, "♣A")
	output := c.RenderStandard52()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
	expected = render.Fg(render.Black, "♣A") + " "
	output = c.RenderStandard52FixedWidth()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
	c = Card{
		Suit: STANDARD_52_SUIT_DIAMONDS,
		Rank: STANDARD_52_RANK_10,
	}
	expected = render.Fg(render.Red, "♦10")
	output = c.RenderStandard52()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
	c = Card{
		Suit: STANDARD_52_SUIT_HEARTS,
		Rank: STANDARD_52_RANK_KING,
	}
	expected = render.Fg(render.Red, "♥K") + " "
	output = c.RenderStandard52FixedWidth()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
	c = Card{
		Suit: STANDARD_52_SUIT_SPADES,
		Rank: STANDARD_52_RANK_QUEEN,
	}
	expected = render.Fg(render.Grey, "##")
	output = RenderStandard52Hidden()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
	expected = render.Fg(render.Grey, "##") + " "
	output = RenderStandard52HiddenFixedWidth()
	if output != expected {
		t.Error("Expected", expected, "but got", output)
	}
}

func TestAceHigh(t *testing.T) {
	d := Standard52DeckAceHigh()
	if d[len(d)-1].Rank <= STANDARD_52_RANK_KING {
		t.Fatal("Expected ace value to be higher than king")
	}
}
