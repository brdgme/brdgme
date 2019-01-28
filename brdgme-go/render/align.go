package render

import "fmt"

type Al byte

const (
	Left Al = iota
	Center
	Right
)

var alignStrs = []string{
	"left",
	"center",
	"right",
}

func Align(a Al, width int, content string) string {
	return fmt.Sprintf("{{align %s %d}}%s{{/align}}", alignStrs[a], width, content)
}
