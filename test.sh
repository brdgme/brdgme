#!/bin/bash
set -e
DIR=$(dirname "$0")
cd "$DIR/brdgme-go"
go test ./...
cd "$DIR"
cargo test
