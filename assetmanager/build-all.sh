#!/usr/bin/env bash

rustup target add wasm32-unknown-unknown --toolchain nightly
pushd administration && cargo +nightly contract build && popd &&
cargo +nightly contract build