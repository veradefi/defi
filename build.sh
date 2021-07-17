#!/bin/bash

cat <<"EOF"
                        __ _                        _       _ 
__   _____ _ __ __ _   / _(_)_ __   __ _ _ __   ___(_) __ _| |
\ \ / / _ \ '__/ _` | | |_| | '_ \ / _` | '_ \ / __| |/ _` | |
 \ V /  __/ | | (_| |_|  _| | | | | (_| | | | | (__| | (_| | |
  \_/ \___|_|  \__,_(_)_| |_|_| |_|\__,_|_| |_|\___|_|\__,_|_|
EOF

echo This will take a while please be patient

DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tzdata

sudo apt update

sudo apt install -y git clang curl libssl-dev llvm libudev-dev

curl https://getsubstrate.io -sSf | bash -s -- --fast

source ~/.cargo/env

cargo install cargo-contract --vers ^0.8 --force --locked

rustup component add rust-src --toolchain nightly

echo Please run the following command
echo source ~/.cargo/env
