package cathedral

import (
	"bytes"
	"errors"
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type PlayCommand struct{}

func (c PlayCommand) Name() string { return "play" }

func (c PlayCommand) Call(
	player string,
	context interface{},
	input *command.Reader,
) (output string, err error) {
	g := context.(*Game)
	pNum, ok := g.PlayerNum(player)
	if !ok {
		return "", errors.New("could not find player")
	}
	args, err := input.ReadLineArgs()
	if err != nil || len(args) < 2 {
		return "", errors.New("the play command requires at least two arguments")
	}
	pieceNum, err := strconv.Atoi(args[0])
	if err != nil {
		return "", errors.New("the first argument should be the piece number to play")
	}
	pieceNum-- // Change to zero index
	loc, ok := ParseLoc(args[1])
	if !ok {
		return "", errors.New("the second argument should be a valid location, such as C7")
	}
	dir := DirDown
	if len(args) > 2 {
		dir, err = helper.MatchStringInStringMap(args[2], OrthoDirNames)
		if err != nil {
			return "", err
		}
	}
	return "", g.Play(pNum, pieceNum, loc, dir)
}

func (c PlayCommand) Usage(player string, context interface{}) string {
	return "{{b}}play # loc (dir){{_b}} to play a tile in a direction, eg. {{b}}play 1 b5 right{{_b}}"
}
*/

func (g *Game) CanPlay(player int) bool {
	if g.NoOpenTiles {
		// Both players play simultaneously.
		return g.CanPlaySomething(player, LocFilterPlayable)
	}
	return g.CurrentPlayer == player
}

func (g *Game) CanPlayPiece(player, piece int, loc Loc, dir Dir) (bool, string) {
	if piece < 0 || piece > len(Pieces[player]) {
		return false, "that is not a valid piece number"
	}
	if g.PlayedPieces[player][piece] {
		return false, "you have already played that piece"
	}
	p := Pieces[player][piece]
	// Special case for player 2, if they haven't played the cathedral they
	// need to play it first.
	if player == 1 && piece != 0 && !g.PlayedPieces[1][0] {
		return false, "cathedral piece must be played before any others"
	}
	n := 0
	switch dir {
	case DirUp:
		n = 2
	case DirRight:
		n = -1
	case DirLeft:
		n = 1
	}
	rotated := p.Positions.Rotate(n)
	// First ensure it can actually be played.
	for _, l := range rotated {
		l = l.Add(loc)
		if !l.Valid() {
			return false, "playing there would go off the board"
		}
		t := g.Board[l.String()]
		if t.Player != NoPlayer {
			return false, "there is already a piece there"
		}
		if t.Owner != NoPlayer &&
			t.Owner != player {
			return false, "the other player owns that area"
		}
	}
	return true, ""
}

func (g *Game) Play(player, piece int, loc Loc, dir Dir) ([]brdgme.Log, error) {
	if !g.CanPlay(player) {
		return nil, errors.New("can't make plays at the moment")
	}
	if ok, reason := g.CanPlayPiece(player, piece, loc, dir); !ok {
		return nil, errors.New(reason)
	}

	logs := []brdgme.Log{}
	p := Pieces[player][piece]
	n := 0
	switch dir {
	case DirUp:
		n = 2
	case DirRight:
		n = -1
	case DirLeft:
		n = 1
	}
	rotated := p.Positions.Rotate(n)
	for _, l := range rotated {
		l = l.Add(loc)
		t := g.Board[l.String()]
		t.Player = p.Player
		t.Type = p.Type
		g.Board[l.String()] = t
	}
	g.PlayedPieces[player][piece] = true
	logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s (size %s) %s from %s",
		render.Player(player),
		render.Bold(strconv.Itoa(p.Type)),
		render.Bold(strconv.Itoa(len(p.Positions))),
		render.Bold(OrthoDirNames[dir]),
		render.Bold(loc.String()),
	)))
	// Do an ownership check.
	if p.Player != PlayerCathedral && g.PlayedPieces[1][0] {
		logs = append(logs, g.CheckCaptures(loc)...)
	}
	// If neither player can play anything, it's the end of the game.
	playablePiece := false
	for p := 0; p < g.Players; p++ {
		playablePiece = g.CanPlaySomething(p, LocFilterPlayable)
		if playablePiece {
			break
		}
	}
	if !playablePiece {
		// The game is finished
		g.Finished = true
		buf := bytes.NewBufferString(render.Bold(
			"The game is finished, remaining piece size is as follows:",
		))
		for p := 0; p < g.Players; p++ {
			buf.WriteString(fmt.Sprintf(
				"\n%s - %s",
				render.Player(p),
				render.Bold(strconv.Itoa(g.RemainingPieceSize(p))),
			))
		}
		logs = append(logs, brdgme.NewPublicLog(buf.String()))
	} else if !g.NoOpenTiles {
		// The game isn't finidhed yet. Check if all open tiles are now used,
		// and if so we switch to simultaneous mode
		openTileExists := false
		for p := 0; p < g.Players; p++ {
			if g.CanPlaySomething(p, LocFilterOpen) {
				openTileExists = true
				break
			}
		}
		if !openTileExists {
			// We don't have any open tiles left, so it becomes simultaneous.
			g.NoOpenTiles = true
			logs = append(logs, brdgme.NewPublicLog(
				"No open tiles remain, players will play the rest of their pieces simultaneously.",
			))
		} else if player != 1 || piece != 0 {
			// Go to next player if it wasn't the cathedral just played.
			g.NextPlayer()
		}
	}
	return logs, nil
}

func (g *Game) CheckCaptures(loc Loc) []brdgme.Log {
	player := g.Board[loc.String()].Player
	// Walk to find all adjoining empty regions.
	visited := map[Loc]bool{}
	capturedTileCount := 0
	capturedPieceCount := 0
	capturedPieceSize := 0
	Walk(loc, OrthoDirs, func(l Loc) int {
		if visited[l] {
			return WalkBlocked
		}
		if g.Board[l.String()].Owner == player {
			// Player already owns it so we don't need to keep walking here.
			visited[l] = true
			return WalkBlocked
		}
		if g.Board[l.String()].Player == player {
			// Extension of the player pieces, continue.
			visited[l] = true
			return WalkContinue
		}
		// Check for capture.
		area := []Loc{}
		pieces := map[PlayerType]bool{}
		Walk(l, Dirs, func(l Loc) int {
			if visited[l] || g.Board[l.String()].Player == player {
				return WalkBlocked
			}
			visited[l] = true
			area = append(area, l)
			if g.Board[l.String()].Player != NoPlayer {
				pieces[g.Board[l.String()].PlayerType] = true
			}
			return WalkContinue
		})
		if len(pieces) <= 1 {
			// Capture!
			capturedTileCount += len(area)
			for pt := range pieces {
				if pt.Player != PlayerCathedral {
					capturedPieceCount++
					g.PlayedPieces[pt.Player][pt.Type-1] = false
				}
			}
			for _, areaLoc := range area {
				if g.Board[areaLoc.String()].Player != NoPlayer &&
					g.Board[areaLoc.String()].Player != PlayerCathedral {
					capturedPieceSize++
				}
				t := EmptyTile
				t.Owner = player
				g.Board[areaLoc.String()] = t
			}
		}
		return WalkContinue
	})
	logs := []brdgme.Log{}
	if capturedTileCount > 0 {
		suffix := ""
		if capturedPieceCount > 0 {
			suffix = fmt.Sprintf(
				" and returned %s pieces with a combined size of %s",
				render.Bold(strconv.Itoa(capturedPieceCount)),
				render.Bold(strconv.Itoa(capturedPieceSize)),
			)
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s captured an area of %s%s",
			render.Player(player),
			render.Bold(strconv.Itoa(capturedTileCount)),
			suffix,
		)))
	}
	return logs
}

var parseLocRegexp = regexp.MustCompile(`(?i)^([a-j])(\d+)$`)

func ParseLoc(input string) (loc Loc, ok bool) {
	matches := parseLocRegexp.FindStringSubmatch(input)
	if matches == nil {
		return
	}
	loc.Y = int(strings.ToUpper(matches[1])[0] - 'A')
	loc.X, _ = strconv.Atoi(matches[2])
	loc.X--
	ok = loc.Valid()
	return
}
