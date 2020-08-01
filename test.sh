#!/bin/bash
set -e
cd "$(dirname "$0")"
docker build --target=rust-test .
docker build --target=go-test .
docker build --target=web-test .
