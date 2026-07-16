package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/modern_art_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &modern_art_1.Game{} })
}
