package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/sushi_go"
)

func main() {
	cmd.Cli(&sushi_go.Game{}, os.Stdin, os.Stdout)
}
