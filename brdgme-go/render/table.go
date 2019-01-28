package render

import (
	"bytes"
	"fmt"
	"strings"
)

type Cell struct {
	Align   Al
	Content string
}

func Cel(content string, align ...Al) Cell {
	al := Left
	if len(align) > 0 {
		al = align[0]
	}
	return Cell{
		Align:   al,
		Content: content,
	}
}

func (c Cell) ToString() string {
	return fmt.Sprintf("{{cell %s}}%s{{/cell}}", alignStrs[c.Align], c.Content)
}

func Table(rows [][]Cell, rowSpacing, colSpacing int) string {
	output := bytes.NewBufferString("{{table}}")
	for i, r := range rows {
		if i != 0 && rowSpacing > 0 {
			output.WriteString(row([]Cell{
				{Align: Left, Content: strings.Repeat("\n", rowSpacing-1)},
			}, 0))
		}
		output.WriteString(row(r, colSpacing))
	}
	output.WriteString("{{/table}}")
	return output.String()
}

func row(cells []Cell, colSpacing int) string {
	output := bytes.NewBufferString("{{row}}")
	for i, c := range cells {
		if i != 0 && colSpacing > 0 {
			output.WriteString(Cell{
				Align:   Left,
				Content: strings.Repeat(" ", colSpacing),
			}.ToString())
		}
		output.WriteString(c.ToString())
	}
	output.WriteString("{{/row}}")
	return output.String()
}
