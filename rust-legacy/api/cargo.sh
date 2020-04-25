#!/bin/bash
set -e

SRC_DIR="$(readlink -e "$(dirname $0)")"
cd "$SRC_DIR"

if [ ! -d "openssl" ]
then
	mkdir -p openssl
	cd openssl
	wget https://www.openssl.org/source/old/1.1.0/openssl-1.1.0l.tar.gz
	tar zxvf openssl-1.1.0l.tar.gz
	cd openssl-1.1.0l
	./config
	make
	cd "$SRC_DIR"
fi

env OPENSSL_LIB_DIR="$SRC_DIR/openssl/openssl-1.1.0l" OPENSSL_INCLUDE_DIR="$SRC_DIR/openssl/openssl-1.1.0l/include" cargo "$@"
