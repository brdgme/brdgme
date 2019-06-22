package render

import (
	"bytes"
	"fmt"
)

// Color is a RGB color
type Color struct {
	R, G, B int
}

// ToString renders the color
func (c Color) ToString() string {
	return fmt.Sprintf("%d,%d,%d", c.R, c.G, c.B)
}

// ColorTrans is a transform flag
type ColorTrans byte

const (
	// Inv is a transform to invert the color
	Inv ColorTrans = iota
	// Mono is a transform to change the color to the nearest monochrome
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

// Fg sets the foreground color
func Fg(c Color, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{fg rgb(%s)%s}}%s{{/fg}}", c.ToString(), colorTransStr(t), content)
}

// Fgp sets the foreground color for a player
func Fgp(player int, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{fg player(%d)%s}}%s{{/fg}}", player, colorTransStr(t), content)
}

// Bg sets the background color
func Bg(c Color, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{bg rgb(%s)%s}}%s{{/bg}}", c.ToString(), colorTransStr(t), content)
}

// Bgp sets the background color for a player
func Bgp(player int, content string, t ...ColorTrans) string {
	return fmt.Sprintf("{{bg player(%d)%s}}%s{{/bg}}", player, colorTransStr(t), content)
}

var (
	// Red Color
	Red = Color{
		R: 211,
		G: 47,
		B: 47,
	}
	// Pink Color
	Pink = Color{
		R: 194,
		G: 24,
		B: 91,
	}
	// Purple Color
	Purple = Color{
		R: 123,
		G: 31,
		B: 162,
	}
	// DeepPurple Color
	DeepPurple = Color{
		R: 81,
		G: 45,
		B: 168,
	}
	// Indigo Color
	Indigo = Color{
		R: 48,
		G: 63,
		B: 159,
	}
	// Blue Color
	Blue = Color{
		R: 25,
		G: 118,
		B: 210,
	}
	// LightBlue Color
	LightBlue = Color{
		R: 2,
		G: 136,
		B: 209,
	}
	// Cyan Color
	Cyan = Color{
		R: 0,
		G: 151,
		B: 167,
	}
	// Teal Color
	Teal = Color{
		R: 0,
		G: 121,
		B: 107,
	}
	// Green Color
	Green = Color{
		R: 56,
		G: 142,
		B: 60,
	}
	// LightGreen Color
	LightGreen = Color{
		R: 104,
		G: 159,
		B: 56,
	}
	// Lime Color
	Lime = Color{
		R: 175,
		G: 180,
		B: 43,
	}
	// Yellow Color
	Yellow = Color{
		R: 251,
		G: 192,
		B: 45,
	}
	// Amber Color
	Amber = Color{
		R: 255,
		G: 160,
		B: 0,
	}
	// Orange Color
	Orange = Color{
		R: 245,
		G: 124,
		B: 0,
	}
	// DeepOrange Color
	DeepOrange = Color{
		R: 230,
		G: 74,
		B: 25,
	}
	// Brown Color
	Brown = Color{
		R: 93,
		G: 64,
		B: 55,
	}
	// Grey Color
	Grey = Color{
		R: 97,
		G: 97,
		B: 97,
	}
	// BlueGrey Color
	BlueGrey = Color{
		R: 69,
		G: 90,
		B: 100,
	}
	// White Color
	White = Color{
		R: 255,
		G: 255,
		B: 255,
	}
	// Black Color
	Black = Color{R: 0, G: 0, B: 0}
)
