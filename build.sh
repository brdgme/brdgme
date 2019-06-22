#!/bin/bash
set -e
DIR=$(dirname "$0")
cd "$DIR/brdgme-go"
go build ./...
cd "$DIR"
cargo build
