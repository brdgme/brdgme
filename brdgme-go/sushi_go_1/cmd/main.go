package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/sushi_go_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &sushi_go_1.Game{} })
}
