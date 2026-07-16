package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/texas_holdem_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &texas_holdem_1.Game{} })
}
