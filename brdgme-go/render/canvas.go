package render

import (
	"bytes"
	"fmt"
)

type Layer struct {
	X, Y    int
	Content string
}

func (l Layer) ToString() string {
	return fmt.Sprintf("{{layer %d %d}}%s{{/layer}}", l.X, l.Y, l.Content)
}

func Canvas(layers []Layer) string {
	output := bytes.NewBufferString("{{canvas}}")
	for _, l := range layers {
		output.WriteString(l.ToString())
	}
	output.WriteString("{{/canvas}}")
	return output.String()
}
