package splendor_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestLevel1Cards(t *testing.T) {
	assert.Len(t, Level1Cards(), 40)
}

func TestLevel2Cards(t *testing.T) {
	assert.Len(t, Level2Cards(), 30)
}

func TestLevel3Cards(t *testing.T) {
	assert.Len(t, Level3Cards(), 20)
}
