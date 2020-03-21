#!/bin/bash
set -e
go vet ./...
cargo clippy -- -D warnings
