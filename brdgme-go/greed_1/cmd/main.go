package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/greed_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &greed_1.Game{} })
}
