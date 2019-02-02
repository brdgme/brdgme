package brdgme

import (
	"testing"

	"brdgme-go/assert"
)

func TestCmpMetrics(t *testing.T) {
	assert.Equal(t, []int{2, 1}, GenPlacings([][]int{{12, 34}, {13, 33}}))
	assert.Equal(t, []int{1, 1}, GenPlacings([][]int{{12, 34}, {12, 34}}))
	assert.Equal(t, []int{1, 2}, GenPlacings([][]int{{12, 36}, {12, 35}}))
	assert.Equal(t, []int{1, 2}, GenPlacings([][]int{{12, 35, 0}, {12, 35}}))
}
