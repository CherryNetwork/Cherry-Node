#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

sudo apt update
sudo apt install -y git clang curl libssl-dev llvm libudev-dev pkg-config

echo "*** Initializing WASM build environment"

if [ -z $CI_PROJECT_NAME ]; then
    rustup update nightly
    rustup update stable
fi

rustup target add wasm32-unknown-unknown --toolchain nightly


cargo b --release && ./target/release/cherry purge-chain --chain cherry-testnet -y && ./target/release/cherry --chain cherry-testnet --bootnodes /ip4/13.38.120.202/tcp/30333/p2p/12D3KooWHETE6VGEBTyFjbrm6yYmKdPYSw26rsT1Wz1Qi3P6HrJZ --telemetry-url "wss://telemetry.polkadot.io/submit/ 0"