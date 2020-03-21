#!/bin/bash
set -e
./build.sh
./test.sh
./lint.sh
