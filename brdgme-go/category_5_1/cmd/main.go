package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/category_5_1"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &category_5_1.Game{} })
}
