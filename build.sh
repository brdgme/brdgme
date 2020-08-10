#!/bin/bash
set -e
export DOCKER_BUILDKIT=1
cd "$(dirname "$0")"
docker build --target=api .
docker build --target=web .
docker build --target=websocket .
docker build --target=acquire .
docker build --target=lost-cities .
docker build --target=age-of-war .
