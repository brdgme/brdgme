#!/bin/bash
set -e
go test ./...
cargo test
