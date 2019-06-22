package render

import "fmt"

// Bold renders bold text
func Bold(content string) string {
	return fmt.Sprintf("{{b}}%s{{/b}}", content)
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
