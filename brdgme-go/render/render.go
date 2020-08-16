package render

import "fmt"

// Bold renders bold text
func Bold(content string) string {
	return fmt.Sprintf("{{b}}%s{{/b}}", content)
}

func BoldIf(content string, when bool) string {
	if when {
		return Bold(content)
	}
	return content
}

// Player renders a player by index
func Player(player int) string {
	return fmt.Sprintf("{{player %d}}", player)
}

// Indent intents some content
func Indent(amount int, content string) string {
	return fmt.Sprintf("{{align %d}}%s{{/align}}", amount, content)
}

// Layout creates a column layout
func Layout(content []string) string {
	rows := make([][]Cell, len(content))
	for k, v := range content {
		rows[k] = []Cell{{
			Align:   Center,
			Content: v,
		}}
	}
	return Table(rows, 0, 0)
}

// Markup applies a foreground color and optional emboldening to content
func Markup(content string, fg Color, bold bool) string {
	inner := Fg(fg, content)
	if bold {
		return Bold(inner)
	}
	return inner
}
