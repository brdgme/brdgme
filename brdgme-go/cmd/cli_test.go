package cmd

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type stubGame struct{}

func (g *stubGame) New(players int) ([]brdgme.Log, error) { return nil, nil }
func (g *stubGame) PubState() interface{}                 { return nil }
func (g *stubGame) PlayerState(player int) interface{}    { return nil }
func (g *stubGame) Command(player int, input string, players []string) (brdgme.CommandResponse, error) {
	return brdgme.CommandResponse{}, nil
}
func (g *stubGame) Status() brdgme.Status               { return brdgme.Status{} }
func (g *stubGame) CommandSpec(player int) *brdgme.Spec { return nil }
func (g *stubGame) PlayerCount() int                    { return 2 }
func (g *stubGame) PlayerCounts() []int                 { return []int{2, 3} }
func (g *stubGame) PubRender() string                   { return "" }
func (g *stubGame) PlayerRender(player int) string      { return "" }
func (g *stubGame) Points() []float32                   { return nil }

func cli(t *testing.T, input string) response {
	t.Helper()
	out := &bytes.Buffer{}
	Cli(&stubGame{}, strings.NewReader(input), out)
	var resp response
	if err := json.Unmarshal(out.Bytes(), &resp); err != nil {
		t.Fatalf("could not decode response %q: %v", out.String(), err)
	}
	if resp.SystemError != nil {
		t.Fatalf("system error: %s", resp.SystemError.Message)
	}
	return resp
}

// serde unit variants arrive as bare JSON strings.
func TestCliPlayerCounts(t *testing.T) {
	resp := cli(t, `"PlayerCounts"`)
	if resp.PlayerCounts == nil {
		t.Fatal("expected PlayerCounts response")
	}
}

func TestCliRules(t *testing.T) {
	resp := cli(t, `"Rules"`)
	if resp.Rules == nil {
		t.Fatal("expected Rules response")
	}
}

func TestCliStructRequest(t *testing.T) {
	resp := cli(t, `{"New":{"players":2}}`)
	if resp.New == nil {
		t.Fatal("expected New response")
	}
}
