FROM rust:1-slim-buster

RUN apt-get update \
  && export DEBIAN_FRONTEND=noninteractive \
  && apt-get install -y \
    cmake pkg-config libssl-dev git \
    build-essential clang libclang-dev \
    gcc curl vim

RUN rustup install nightly \
  && rustup target add wasm32-unknown-unknown --toolchain nightly \
  && rustup component add rust-src --toolchain nightly

RUN apt-get update \
    && apt-get install devscripts -y \
    && dget https://deb.debian.org/debian/pool/main/b/binaryen/binaryen_99-3.dsc

RUN cargo install cargo-contract --vers ^0.8 --force --locked

ENTRYPOINT ["/bin/bash"]
