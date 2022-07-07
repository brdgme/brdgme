package cathedral_1

import (
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/assert"
)

func TestPlay_Capture(t *testing.T) {
	g, err := parseGame(`
............G1......
..C1......G1G1G1G1..
C1C1C1..G1G1....G1..
..C1......G1R2..G1..
..C1......G2....G1..
..........G2G2G2....
....................
....................
..R1R1..............
....................
`)
	assert.NoError(t, err)
	assert.True(t, g.PlayedPieces[1][1])
	_, err = g.Command(Mick, "play 9 f9 down", []string{})
	assert.NoError(t, err)
	assertBoard(t, `
............G1G.G.G.
..C1......G1G1G1G1G.
C1C1C1..G1G1G.G.G1G.
..C1......G1G.G.G1G.
..C1......G2G.G.G1G.
..........G2G2G2G9G.
................G9G9
....................
..R1R1..............
....................
`, g.Board)
	assert.False(t, g.PlayedPieces[1][1])
}

func TestPlay_CaptureWithOnePiece(t *testing.T) {
	g, err := parseGame(`
R1..................
G1C1................
C1C1C1..............
..C1................
..C1................
....................
....................
....................
....................
....................
`)
	assert.NoError(t, err)
	_, err = g.Command(Mick, "play 4 a9 down", []string{})
	assert.NoError(t, err)
	assertBoard(t, `
R1..............G4G4
G1C1............G4G.
C1C1C1..........G4G4
..C1................
..C1................
....................
....................
....................
....................
....................
`, g.Board)
}