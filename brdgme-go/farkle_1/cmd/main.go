package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/farkle_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &farkle_1.Game{} })
}
