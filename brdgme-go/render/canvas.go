package render

import (
	"bytes"
	"fmt"
)

// Layer is a layer in a canvas
type Layer struct {
	X, Y    int
	Content string
}

// ToString renders a layer
func (l Layer) ToString() string {
	return fmt.Sprintf("{{layer %d %d}}%s{{/layer}}", l.X, l.Y, l.Content)
}

// Canvas renders content in positioned layers
func Canvas(layers []Layer) string {
	output := bytes.NewBufferString("{{canvas}}")
	for _, l := range layers {
		output.WriteString(l.ToString())
	}
	output.WriteString("{{/canvas}}")
	return output.String()
}
