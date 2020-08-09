#!/bin/bash
set -e
cd "$(dirname "$0")"
docker build --target=api .
docker build --target=web .
docker build --target=websocket .
docker build --target=acquire .
docker build --target=lost-cities .
docker build --target=age-of-war .
