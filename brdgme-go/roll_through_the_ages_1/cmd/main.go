package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/roll_through_the_ages_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &roll_through_the_ages_1.Game{} })
}
