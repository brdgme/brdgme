package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cathedral_1"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &cathedral_1.Game{} })
}
