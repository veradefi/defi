#!/bin/bash

cat <<"EOF"
                        __ _                        _       _ 
__   _____ _ __ __ _   / _(_)_ __   __ _ _ __   ___(_) __ _| |
\ \ / / _ \ '__/ _` | | |_| | '_ \ / _` | '_ \ / __| |/ _` | |
 \ V /  __/ | | (_| |_|  _| | | | | (_| | | | | (__| | (_| | |
  \_/ \___|_|  \__,_(_)_| |_|_| |_|\__,_|_| |_|\___|_|\__,_|_|
EOF

echo This will take a while please be patient


if [ -n "( lsb_release -a | grep Ubuntu ) " ];then

DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tzdata

sudo apt update

sudo apt install -y git clang curl libssl-dev llvm libudev-dev

if [ -z "$(command -v rustc)" ]; then

curl https://getsubstrate.io -sSf | bash -s -- --fast

fi

source ~/.cargo/env

if [ "$(cargo-contract -V)" != "cargo-contract 0.8.0" ]; then

cargo install cargo-contract --vers ^0.8 --force --locked

fi

rustup component add rust-src --toolchain nightly

for d in */ ; do
    (cd "$d" &&
    if [ -f Cargo.toml ]; then
    echo "Building ----> $d";
    cargo +nightly contract build
    fi)
done

fi
