package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/love_letter_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &love_letter_1.Game{} })
}
