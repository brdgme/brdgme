package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/no_thanks_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &no_thanks_1.Game{} })
}
