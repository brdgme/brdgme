package main

import (
	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/zombie_dice_1"
)

func main() {
	cmd.Serve(func() brdgme.Gamer { return &zombie_dice_1.Game{} })
}
