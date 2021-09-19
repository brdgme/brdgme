package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/sushi_go_1"
)

func main() {
	cmd.Cli(&sushi_go_1.Game{}, os.Stdin, os.Stdout)
}
