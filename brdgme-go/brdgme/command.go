package brdgme

import (
	"bytes"
	"encoding/json"
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

type Parser interface {
	Parse(input string, names []string) (Output, *ParseError)
	Expected(names []string) []string
	ToSpec() Spec
}

type Output struct {
	Value     interface{}
	Consumed  string
	Remaining string
}

type ParseError struct {
	Message  string
	Expected []string
	Offset   uint
}

func (e ParseError) Error() string {
	output := &bytes.Buffer{}
	if e.Message != "" {
		output.WriteString(e.Message)
		if len(e.Expected) > 0 {
			output.WriteString(", ")
		}
	}
	if len(e.Expected) > 0 {
		output.WriteString("expected ")
		output.WriteString(commaListOr(e.Expected))
	}
	return output.String()
}

type specPlain Spec // To avoid infinite loop when marshalling

type Spec struct {
	Int    *Int       `json:",omitempty"`
	Token  *Token     `json:",omitempty"`
	Enum   *EnumSpec  `json:",omitempty"`
	OneOf  *OneOfSpec `json:",omitempty"`
	Chain  *ChainSpec `json:",omitempty"`
	Many   *ManySpec  `json:",omitempty"`
	Opt    *OptSpec   `json:",omitempty"`
	Doc    *DocSpec   `json:",omitempty"`
	Player *Player    `json:",omitempty"`
	Space  *Space     `json:",omitempty"`
}

// MarshalJSON is a custom implementation to just use a string for Player and
// Space.
func (s Spec) MarshalJSON() ([]byte, error) {
	if s.Player != nil {
		return json.Marshal("Player")
	}
	if s.Space != nil {
		return json.Marshal("Space")
	}
	return json.Marshal(specPlain(s))
}

type Int struct {
	Min *int `json:"min,omitempty"`
	Max *int `json:"max,omitempty"`
}

var _ Parser = Int{}

func (i Int) ToSpec() Spec {
	return Spec{
		Int: &i,
	}
}

var IntRegexp = regexp.MustCompile("^-?[0-9]+")

func (i Int) Parse(input string, names []string) (Output, *ParseError) {
	match := IntRegexp.FindString(input)
	parsed, err := strconv.Atoi(match)
	if err != nil {
		return Output{}, &ParseError{
			Expected: i.Expected(names),
		}
	}
	if i.Min != nil && int(*i.Min) > parsed {
		return Output{}, &ParseError{
			Message:  fmt.Sprintf("%d is too low", parsed),
			Expected: i.Expected(names),
		}
	}
	if i.Max != nil && int(*i.Max) < parsed {
		return Output{}, &ParseError{
			Message:  fmt.Sprintf("%d is too high", parsed),
			Expected: i.Expected(names),
		}
	}
	return Output{
		Value:     parsed,
		Consumed:  match,
		Remaining: input[len(match):],
	}, nil
}

func (i Int) ExpectedOutput() string {
	switch {
	case i.Min != nil && i.Max != nil:
		return fmt.Sprintf("number between %d and %d", *i.Min, *i.Max)
	case i.Min != nil && i.Max == nil:
		return fmt.Sprintf("number %d or higher", *i.Min)
	case i.Max != nil && i.Min == nil:
		return fmt.Sprintf("number %d or lower", *i.Max)
	default:
		return "number"
	}
}

func (i Int) Expected(names []string) []string {
	return []string{i.ExpectedOutput()}
}

type Token string

var _ Parser = Token("")

func (t Token) ToSpec() Spec {
	return Spec{
		Token: &t,
	}
}

func (t Token) Parse(input string, names []string) (Output, *ParseError) {
	tLen := len(t)
	if len(input) >= tLen &&
		strings.HasPrefix(strings.ToLower(input[:tLen]), strings.ToLower(string(t))) {
		return Output{
			Value:     t,
			Consumed:  input[:tLen],
			Remaining: input[tLen:],
		}, nil
	}
	return Output{}, &ParseError{
		Expected: t.Expected(names),
	}
}

func (t Token) Expected(names []string) []string {
	return []string{string(t)}
}

type EnumValue struct {
	Name  string
	Value interface{}
}

type Enum struct {
	Values []EnumValue
	Exact  bool
}

var _ Parser = Enum{}

type EnumSpec struct {
	Values []string `json:"values"`
	Exact  bool     `json:"exact"`
}

func EnumFromStrings(values []string, exact bool) Enum {
	evs := make([]EnumValue, len(values))
	for k, v := range values {
		evs[k] = EnumValue{
			Name:  v,
			Value: v,
		}
	}
	return Enum{
		Values: evs,
		Exact:  exact,
	}
}

func EnumFromInts(values []int, exact bool) Enum {
	evs := make([]EnumValue, len(values))
	for k, v := range values {
		evs[k] = EnumValue{
			Name:  strconv.Itoa(v),
			Value: v,
		}
	}
	return Enum{
		Values: evs,
		Exact:  exact,
	}
}

func (e Enum) ToSpec() Spec {
	values := []string{}
	for _, v := range e.Values {
		values = append(values, v.Name)
	}
	return Spec{
		Enum: &EnumSpec{
			Values: values,
			Exact:  e.Exact,
		},
	}
}

func sharedPrefix(s1, s2 string) int {
	until := len(s1)
	if s2Len := len(s2); s2Len < until {
		until = s2Len
	}

	for i := 0; i < until; i++ {
		if s1[i] != s2[i] {
			return i
		}
	}

	return until
}

func commaList(items []string, conj string) string {
	switch len(items) {
	case 0:
		return ""
	case 1:
		return items[0]
	case 2:
		return fmt.Sprintf("%s %s %s", items[0], conj, items[1])
	default:
		return fmt.Sprintf("%s, %s", items[0], commaList(items[1:], conj))
	}
}

func commaListAnd(items []string) string {
	return commaList(items, "and")
}

func commaListOr(items []string) string {
	return commaList(items, "or")
}

func (e Enum) Parse(input string, names []string) (Output, *ParseError) {
	inputLower := strings.ToLower(input)
	matchedKeys := []EnumValue{}
	matchLen := 0
	fullMatch := false

	for _, v := range e.Values {
		vLower := strings.ToLower(v.Name)
		vLen := len(vLower)

		matching := sharedPrefix(inputLower, vLower)
		if e.Exact && matching < vLen {
			continue
		}

		if matching > 0 && matching >= matchLen {
			curFullMatch := matching == vLen
			if matching > matchLen || (!fullMatch && curFullMatch) {
				matchedKeys = []EnumValue{}
				matchLen = matching
				fullMatch = curFullMatch
			}
			if matching == matchLen && (curFullMatch || !fullMatch) {
				matchedKeys = append(matchedKeys, v)
			}
		}
	}

	switch len(matchedKeys) {
	case 1:
		return Output{
			Value:     matchedKeys[0].Value,
			Consumed:  input[:matchLen],
			Remaining: input[matchLen:],
		}, nil
	case 0:
		return Output{}, &ParseError{
			Expected: e.Expected(names),
		}
	default:
		return Output{}, &ParseError{
			Message: fmt.Sprintf(
				"matched %s, more input is required to uniquely match one",
				commaListAnd(enumValueNames(matchedKeys)),
			),
			Expected: e.Expected(names),
		}
	}
}

func enumValueNames(ev []EnumValue) []string {
	names := []string{}
	for _, v := range ev {
		names = append(names, v.Name)
	}
	return names
}

func (e Enum) names() []string {
	return enumValueNames(e.Values)
}

func (e Enum) Expected(names []string) []string {
	return e.names()
}

func parsersToSpecs(parsers []Parser) []Spec {
	specs := make([]Spec, len(parsers))
	for k, v := range parsers {
		specs[k] = v.ToSpec()
	}
	return specs
}

type OneOf []Parser

var _ Parser = OneOf{}

type OneOfSpec []Spec

func (o OneOf) ToSpec() Spec {
	spec := OneOfSpec(parsersToSpecs(o))
	return Spec{
		OneOf: &spec,
	}
}

func (o OneOf) Parse(input string, names []string) (Output, *ParseError) {
	errors := []ParseError{}
	errorConsumed := uint(0)

	for _, p := range o {
		output, err := p.Parse(input, names)
		if err == nil {
			return output, nil
		}
		if err.Offset > errorConsumed {
			errors = []ParseError{*err}
			errorConsumed = err.Offset
		} else if err.Offset == errorConsumed {
			errors = append(errors, *err)
		}
	}

	messages := []string{}
	expected := []string{}
	for _, e := range errors {
		expected = append(expected, e.Expected...)
		if e.Message != "" {
			messages = append(messages, e.Message)
		}
	}
	return Output{}, &ParseError{
		Message:  commaListOr(messages),
		Expected: expected,
		Offset:   errorConsumed,
	}
}

func (o OneOf) Expected(names []string) []string {
	expected := []string{}
	for _, spec := range o {
		expected = append(expected, spec.Expected(names)...)
	}
	return expected
}

type Chain []Parser

var _ Parser = Chain{}

type ChainSpec []Spec

func (c Chain) ToSpec() Spec {
	spec := ChainSpec(parsersToSpecs(c))
	return Spec{
		Chain: &spec,
	}
}

func (c Chain) Expected(names []string) []string {
	if len(c) == 0 {
		return []string{}
	}
	return c[0].Expected(names)
}

func (c Chain) Parse(input string, names []string) (Output, *ParseError) {
	return parseChain(input, names, c)
}

func parseChain(input string, names []string, parsers []Parser) (Output, *ParseError) {
	pLen := len(parsers)
	if pLen == 0 {
		return Output{
			Value:     []interface{}{},
			Consumed:  "",
			Remaining: input,
		}, nil
	}

	headOutput, headErr := parsers[0].Parse(input, names)
	outputValue := []interface{}{headOutput.Value}
	if headErr != nil {
		headOutput.Value = outputValue
		return headOutput, headErr
	}

	tailOutput, tailErr := parseChain(headOutput.Remaining, names, parsers[1:])
	outputValue = append(outputValue, tailOutput.Value.([]interface{})...)

	if tailErr != nil {
		tailErr.Offset += uint(len(headOutput.Consumed))
	}

	tailOutput.Value = outputValue
	tailOutput.Consumed = headOutput.Consumed + tailOutput.Consumed
	return tailOutput, tailErr
}

type Many struct {
	Parser Parser
	Min    *uint
	Max    *uint
	Delim  Parser
}

var _ Parser = Many{}

type ManySpec struct {
	Spec  Spec  `json:"spec"`
	Min   *uint `json:"min,omitempty"`
	Max   *uint `json:"max,omitempty"`
	Delim *Spec `json:"delim,omitempty"`
}

func (m Many) ToSpec() Spec {
	var delimSpec *Spec
	if m.Delim != nil {
		dSpec := m.Delim.ToSpec()
		delimSpec = &dSpec
	}
	return Spec{
		Many: &ManySpec{
			Spec:  m.Parser.ToSpec(),
			Min:   m.Min,
			Max:   m.Max,
			Delim: delimSpec,
		},
	}
}

func (m Many) ExpectedPrefix() string {
	switch {
	case m.Min != nil && m.Max != nil:
		return fmt.Sprintf("between %d and %d", *m.Min, *m.Max)
	case m.Min != nil:
		return fmt.Sprintf("%d or more", *m.Min)
	case m.Max != nil:
		return fmt.Sprintf("up to %d", *m.Max)
	default:
		return "any number of"
	}
}

func (m Many) Expected(names []string) []string {
	expected := []string{}
	prefix := m.ExpectedPrefix()
	for _, e := range m.Parser.Expected(names) {
		expected = append(expected, fmt.Sprintf("%s %s", prefix, e))
	}
	return expected
}

func (m Many) Parse(input string, names []string) (Output, *ParseError) {
	parsed := []interface{}{}
	if m.Max != nil && (*m.Max == 0 || m.Min != nil && *m.Min > *m.Max) {
		return Output{
			Value:     parsed,
			Remaining: input,
		}, nil
	}

	first := true
	offset := 0

	for {
		innerOffset := offset

		if !first && m.Delim != nil {
			delimOutput, delimErr := m.Delim.Parse(input[offset:], names)
			if delimErr != nil {
				break
			}
			innerOffset += len(delimOutput.Consumed)
		}
		first = false

		specOutput, specErr := m.Parser.Parse(input[innerOffset:], names)
		if specErr != nil {
			break
		}
		parsed = append(parsed, specOutput.Value)
		offset = innerOffset + len(specOutput.Consumed)

		if m.Max != nil && uint(len(parsed)) == *m.Max {
			break
		}
	}

	if m.Min != nil && uint(len(parsed)) < *m.Min {
		return Output{}, &ParseError{
			Message: fmt.Sprintf(
				"expected at least %d items but could only parse %d",
				*m.Min,
				len(parsed),
			),
			Offset: uint(offset),
		}
	}

	return Output{
		Value:     parsed,
		Consumed:  input[:offset],
		Remaining: input[offset:],
	}, nil
}

type Opt struct {
	Parser
}

var _ Parser = Opt{Parser: Token("blah")}

type OptSpec Spec

func (o Opt) ToSpec() Spec {
	spec := OptSpec(o.Parser.ToSpec())
	return Spec{
		Opt: &spec,
	}
}

func (o Opt) Expected(names []string) []string {
	expected := []string{}
	for _, e := range Parser(o).Expected(names) {
		expected = append(expected, fmt.Sprintf("optional %s", e))
	}
	return expected
}

func (o Opt) Parse(input string, names []string) (Output, *ParseError) {
	output, err := o.Parser.Parse(input, names)
	if err != nil {
		return Output{
			Value:     nil,
			Remaining: input,
		}, nil
	}
	return output, err
}

type Doc struct {
	Name   string
	Desc   string
	Parser Parser
}

var _ Parser = Doc{}

type DocSpec struct {
	Name string `json:"name"`
	Desc string `json:"desc"`
	Spec Spec   `json:"spec"`
}

func (d Doc) ToSpec() Spec {
	return Spec{
		Doc: &DocSpec{
			Name: d.Name,
			Desc: d.Desc,
			Spec: d.Parser.ToSpec(),
		},
	}
}

func (d Doc) Expected(names []string) []string {
	return d.Parser.Expected(names)
}

func (d Doc) Parse(input string, names []string) (Output, *ParseError) {
	return d.Parser.Parse(input, names)
}

type Player struct{}

var _ Parser = Player{}

func (p Player) nameEnumValues(names []string) []EnumValue {
	evs := make([]EnumValue, len(names))
	for k, v := range names {
		evs[k] = EnumValue{
			Name:  v,
			Value: k,
		}
	}
	return evs
}

func (p Player) Parser(names []string) Enum {
	return Enum{
		Values: p.nameEnumValues(names),
	}
}

func (p Player) ToSpec() Spec {
	return Spec{
		Player: &p,
	}
}

func (p Player) Expected(names []string) []string {
	return p.Parser(names).Expected(names)
}

func (p Player) Parse(input string, names []string) (Output, *ParseError) {
	return p.Parser(names).Parse(input, names)
}

type Space struct{}

var _ Parser = Space{}

func (s Space) ToSpec() Spec {
	return Spec{
		Space: &s,
	}
}

func (s Space) Expected(names []string) []string {
	return []string{"whitespace"}
}

var SpaceRegexp = regexp.MustCompile(`^(\s+)`)

func (s Space) Parse(input string, names []string) (Output, *ParseError) {
	match := SpaceRegexp.FindString(input)
	if match == "" {
		return Output{
				Value:     "",
				Consumed:  "",
				Remaining: input,
			}, &ParseError{
				Message:  "expected whitespace",
				Expected: s.Expected(names),
			}
	}
	return Output{
		Value:     match,
		Consumed:  match,
		Remaining: input[len(match):],
	}, nil
}

func AfterSpace(parser Parser) Map {
	return Map{
		Parser: Chain{Space{}, parser},
		Func: func(value interface{}) interface{} {
			return value.([]interface{})[1]
		},
	}
}

type Map struct {
	Parser Parser
	Func   func(value interface{}) interface{}
}

var _ Parser = Map{}

func (m Map) ToSpec() Spec {
	return m.Parser.ToSpec()
}

func (m Map) Expected(names []string) []string {
	return m.Parser.Expected(names)
}

func (m Map) Parse(input string, names []string) (Output, *ParseError) {
	o, err := m.Parser.Parse(input, names)
	if err == nil {
		o.Value = m.Func(o.Value)
	}
	return o, err
}
