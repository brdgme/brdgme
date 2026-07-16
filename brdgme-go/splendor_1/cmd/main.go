package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/splendor_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &splendor_1.Game{} })
}
