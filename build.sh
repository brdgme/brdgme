#!/bin/bash
set -e
cd "$(dirname "$0")"
DOCKER_BUILDKIT=1 docker build --target=api .
DOCKER_BUILDKIT=1 docker build --target=web .
DOCKER_BUILDKIT=1 docker build --target=websocket .
DOCKER_BUILDKIT=1 docker build --target=acquire .
DOCKER_BUILDKIT=1 docker build --target=lost_cities .
DOCKER_BUILDKIT=1 docker build --target=age_of_war .
