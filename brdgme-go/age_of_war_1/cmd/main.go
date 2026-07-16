package main

import (
	"github.com/brdgme/brdgme/brdgme-go/age_of_war_1"
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &age_of_war_1.Game{} })
}
