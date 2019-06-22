package render

import "fmt"

// Al is an alignment flag
type Al byte

const (
	// Left alignment
	Left Al = iota
	// Center alignment
	Center
	// Right alignment
	Right
)

var alignStrs = []string{
	"left",
	"center",
	"right",
}

// Align aligns some content
func Align(a Al, width int, content string) string {
	return fmt.Sprintf("{{align %s %d}}%s{{/align}}", alignStrs[a], width, content)
}
