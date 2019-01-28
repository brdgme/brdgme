package render

import "fmt"

func Bold(content string) string {
	return fmt.Sprintf("{{b}}%s{{/b}}", content)
}

func Player(player int) string {
	return fmt.Sprintf("{{player %d}}", player)
}

func Indent(amount int, content string) string {
	return fmt.Sprintf("{{align %d}}%s{{/align}}")
}

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
