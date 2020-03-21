#!/bin/bash
set -e
cd "$(dirname "$0")"
go test ./...
cargo test
