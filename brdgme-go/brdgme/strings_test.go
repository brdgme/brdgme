package brdgme

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestCommaList(t *testing.T) {
	assert.Equal(t, "one, two and three", CommaList([]string{"one", "two", "three"}))
}
