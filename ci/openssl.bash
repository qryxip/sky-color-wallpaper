#!/bin/bash

OPENSSL_VERSION=1.0.2r

export CC=/usr/bin/musl-gcc
export C_INCLUDE_PATH==/usr/local/musl/include

sudo mkdir -p "$C_INCLUDE_PATH"
sudo ln -s /usr/include/linux "$C_INCLUDE_PATH/"
sudo ln -s /usr/include/x86_64-linux-gnu/asm "$C_INCLUDE_PATH/"
sudo ln -s /usr/include/asm-generic "$C_INCLUDE_PATH/"

curl -sS "https://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz" --retry 10 | tar -xzC /tmp

cd "/tmp/openssl-$OPENSSL_VERSION"
./Configure --prefix=/usr/local/openssl linux-x86_64
make
sudo make install

cat <<EOF >>"$GITHUB_ENV"
PKG_CONFIG_ALLOW_CROSS=1
OPENSSL_DIR=/usr/local/openssl
OPENSSL_STATIC=1
EOF
