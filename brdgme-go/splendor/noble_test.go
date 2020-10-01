package splendor

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestNobleCards(t *testing.T) {
	assert.Len(t, NobleCards(), 10)
}
