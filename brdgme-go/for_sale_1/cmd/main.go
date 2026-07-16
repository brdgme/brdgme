package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/for_sale_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &for_sale_1.Game{} })
}
