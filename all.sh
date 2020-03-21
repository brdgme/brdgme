#!/bin/bash
set -e
cd "$(dirname "$0")"
./build.sh
./test.sh
./lint.sh
