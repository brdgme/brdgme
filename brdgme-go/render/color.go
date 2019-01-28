package render

import (
	"bytes"
	"fmt"
)

type Color struct {
	R, G, B int
}

func (c Color) ToString() string {
	return fmt.Sprintf("%d,%d,%d", c.R, c.G, c.B)
}

type ColorTrans byte

const (
	Inv ColorTrans = iota
	Mono
)

var colorTransStrs = []string{
	"inv",
	"mono",
}

func colorTransStr(t []ColorTrans) string {
	output := bytes.Buffer{}
	for _, ct := range t {
		output.WriteString(fmt.Sprintf(" | %s", colorTransStrs[ct]))
	}
	return output.String()
}

func Fg(c Color, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{fg rgb(%s)%s}}%s{{/fg}}", c.ToString(), colorTransStr(t), content)
}

func Fgp(player int, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{fg player(%d)%s}}%s{{/fg}}", player, colorTransStr(t), content)
}

func Bg(c Color, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{bg rgb(%s)%s}}%s{{/bg}}", c.ToString(), colorTransStr(t), content)
}

func Bgp(player int, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{bg player(%d)%s}}%s{{/bg}}", player, colorTransStr(t), content)
}

var (
	Red = Color{
		R: 211,
		G: 47,
		B: 47,
	}
	Pink = Color{
		R: 194,
		G: 24,
		B: 91,
	}
	Purple = Color{
		R: 123,
		G: 31,
		B: 162,
	}
	DeepPurple = Color{
		R: 81,
		G: 45,
		B: 168,
	}
	Indigo = Color{
		R: 48,
		G: 63,
		B: 159,
	}
	Blue = Color{
		R: 25,
		G: 118,
		B: 210,
	}
	LightBlue = Color{
		R: 2,
		G: 136,
		B: 209,
	}
	Cyan = Color{
		R: 0,
		G: 151,
		B: 167,
	}
	Teal = Color{
		R: 0,
		G: 121,
		B: 107,
	}
	Green = Color{
		R: 56,
		G: 142,
		B: 60,
	}
	LightGreen = Color{
		R: 104,
		G: 159,
		B: 56,
	}
	Lime = Color{
		R: 175,
		G: 180,
		B: 43,
	}
	Yellow = Color{
		R: 251,
		G: 192,
		B: 45,
	}
	Amber = Color{
		R: 255,
		G: 160,
		B: 0,
	}
	Orange = Color{
		R: 245,
		G: 124,
		B: 0,
	}
	DeepOrange = Color{
		R: 230,
		G: 74,
		B: 25,
	}
	Brown = Color{
		R: 93,
		G: 64,
		B: 55,
	}
	Grey = Color{
		R: 97,
		G: 97,
		B: 97,
	}
	BlueGrey = Color{
		R: 69,
		G: 90,
		B: 100,
	}
	White = Color{
		R: 255,
		G: 255,
		B: 255,
	}
	Black = Color{R: 0, G: 0, B: 0}
)
