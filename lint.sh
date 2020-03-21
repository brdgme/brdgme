#!/bin/bash
set -e
cd "$(dirname "$0")"
go vet ./...
cargo clippy -- -D warnings
