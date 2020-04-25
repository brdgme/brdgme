#!/bin/bash
set -e
cd "$(dirname "$0")"
go vet ./...
cd rust && cargo clippy -- -D warnings
