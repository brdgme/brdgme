#!/bin/bash
set -e
DIR=$(dirname "$0")
cd "$DIR/brdgme-go"
golangci-lint run
cd "$DIR"
cargo clippy -- -D warnings
