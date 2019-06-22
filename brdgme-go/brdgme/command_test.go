package brdgme

import (
	"testing"

	"github.com/brdgme/brdgme-go/assert"
)

func TestInt(t *testing.T) {
	parser := Int{}
	output, err := parser.Parse("57 cheese and bacon", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     57,
		Consumed:  "57",
		Remaining: " cheese and bacon",
	}, output)

	output, err = parser.Parse("-31 dacon", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     -31,
		Consumed:  "-31",
		Remaining: " dacon",
	}, output)

	_, err = parser.Parse("bleh moo", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"number"},
	}, *err)

	min := 3
	parser.Min = &min
	output, err = parser.Parse("3 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     3,
		Consumed:  "3",
		Remaining: " blah",
	}, output)

	output, err = parser.Parse("4 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     4,
		Consumed:  "4",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("2 blah", []string{})
	assert.Equal(t, ParseError{
		Message:  "2 is too low",
		Expected: []string{"number 3 or higher"},
	}, *err)

	max := 5
	parser.Max = &max

	output, err = parser.Parse("4 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     4,
		Consumed:  "4",
		Remaining: " blah",
	}, output)

	output, err = parser.Parse("5 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     5,
		Consumed:  "5",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("6 blah", []string{})
	assert.Equal(t, ParseError{
		Message:  "6 is too high",
		Expected: []string{"number between 3 and 5"},
	}, *err)
}

func TestToken(t *testing.T) {
	parser := Token("play")

	output, err := parser.Parse("PlAy blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     Token("play"),
		Consumed:  "PlAy",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("Pley blah", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"play"},
	}, *err)

	_, err = parser.Parse("pl", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"play"},
	}, *err)
}

func TestEnum(t *testing.T) {
	parser := EnumFromStrings([]string{"one", "onetwo", "three"}, false)

	output, err := parser.Parse("One blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     "one",
		Consumed:  "One",
		Remaining: " blah",
	}, output)

	output, err = parser.Parse("thr blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     "three",
		Consumed:  "thr",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("on blah", []string{})
	assert.Equal(t, ParseError{
		Message:  "matched one and onetwo, more input is required to uniquely match one",
		Expected: []string{"one", "onetwo", "three"},
	}, *err)

	parser.Exact = true
	_, err = parser.Parse("thr blah", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"one", "onetwo", "three"},
	}, *err)
}

func TestOneOf(t *testing.T) {
	parser := OneOf{
		Token("egg"),
		Int{},
		Token("cheese"),
	}

	output, err := parser.Parse("cheese blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     Token("cheese"),
		Consumed:  "cheese",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("cheeze blah", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"egg", "number", "cheese"},
	}, *err)
}

func TestChain(t *testing.T) {
	parser := Chain{
		Token("egg"),
		Int{},
		Token("cheese"),
	}

	output, err := parser.Parse("egg5cheese blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     []interface{}{Token("egg"), 5, Token("cheese")},
		Consumed:  "egg5cheese",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("cheeze blah", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"egg"},
	}, *err)
}

func TestMany(t *testing.T) {
	parser := Many{
		Parser: Int{},
	}

	output, err := parser.Parse("1 2 3 4 5 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     []interface{}{1, 2, 3, 4, 5},
		Consumed:  "1 2 3 4 5",
		Remaining: " blah",
	}, output)
}

func TestOpt(t *testing.T) {
	parser := Opt{Int{}}

	output, err := parser.Parse("5 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     5,
		Consumed:  "5",
		Remaining: " blah",
	}, output)

	output, err = parser.Parse("egg 5 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     nil,
		Consumed:  "",
		Remaining: "egg 5 blah",
	}, output)
}

func TestDoc(t *testing.T) {
	parser := Doc{
		Name:   "blah",
		Parser: Int{},
	}

	output, err := parser.Parse("5 blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     5,
		Consumed:  "5",
		Remaining: " blah",
	}, output)

	_, err = parser.Parse("egg 5 blah", []string{})
	assert.Equal(t, ParseError{
		Expected: []string{"number"},
	}, *err)
}

func TestPlayer(t *testing.T) {
	parser := Player{}

	output, err := parser.Parse("bacon blah", []string{"beefsack", "baconheist"})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     1,
		Consumed:  "bacon",
		Remaining: " blah",
	}, output)
}

func TestSpace(t *testing.T) {
	parser := Space{}

	output, err := parser.Parse("    egg", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     "    ",
		Consumed:  "    ",
		Remaining: "egg",
	}, output)

	_, err = parser.Parse("egg", []string{})
	assert.Equal(t, ParseError{
		Message:  "expected whitespace",
		Expected: []string{"whitespace"},
	}, *err)
}

func TestAfterSpace(t *testing.T) {
	parser := AfterSpace(Int{})

	output, err := parser.Parse("    1blah", []string{})
	assert.Nil(t, err)
	assert.Equal(t, Output{
		Value:     1,
		Consumed:  "    1",
		Remaining: "blah",
	}, output)

	_, err = parser.Parse("egg", []string{})
	assert.Equal(t, ParseError{
		Message:  "expected whitespace",
		Expected: []string{"whitespace"},
	}, *err)
}
