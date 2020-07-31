#!/bin/bash
set -e
cd "$(dirname "$0")"
go test ./...
cd rust && cargo test
