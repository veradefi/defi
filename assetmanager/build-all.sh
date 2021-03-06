#!/usr/bin/env bash

pushd administration && cargo +nightly contract build && popd &&
pushd lendingmanager && cargo +nightly contract build && popd &&
cargo +nightly contract build