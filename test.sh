#!/bin/bash
set -e
cd "$(dirname "$0")"
DOCKER_BUILDKIT=1 docker build --target=rust-test .
DOCKER_BUILDKIT=1 docker build --target=go-test .
DOCKER_BUILDKIT=1 docker build --target=web-test .
