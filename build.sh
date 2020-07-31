#!/bin/bash
set -e
cd "$(dirname "$0")"
go build ./...
cd rust && cargo build
