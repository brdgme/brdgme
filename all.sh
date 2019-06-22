#!/bin/bash
set -e
DIR=$(dirname "$0")
cd "$DIR"
./build.sh
./test.sh
./lint.sh
