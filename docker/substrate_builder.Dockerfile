# This is the build stage for Substrate. Here we create the binary.
FROM rust:1.59 as builder

RUN apt update && apt install -y git clang curl libssl-dev llvm libudev-dev libc-bin

RUN rustup default stable
RUN rustup update
RUN rustup update nightly
RUN rustup target add wasm32-unknown-unknown --toolchain nightly

WORKDIR /substrate
COPY . /substrate

RUN cargo build --release

# This is the 2nd stage: a very small image where we copy the Substrate binary."
FROM docker.io/library/ubuntu:20.04
LABEL description="Multistage Docker image for Substrate: a platform for web3" \
    io.parity.image.type="builder" \
    io.parity.image.authors="chevdor@gmail.com, devops-team@parity.io" \
    io.parity.image.vendor="Parity Technologies" \
    io.parity.image.description="Substrate is a next-generation framework for blockchain innovation 🚀" \
    io.parity.image.source="https://github.com/paritytech/polkadot/blob/${VCS_REF}/docker/substrate_builder.Dockerfile" \
    io.parity.image.documentation="https://github.com/paritytech/polkadot/"

COPY --from=builder /substrate/target/release/cherry /usr/local/bin
COPY --from=builder /substrate/target/release/subkey /usr/local/bin
COPY --from=builder /substrate/target/release/chain-spec-builder /usr/local/bin

RUN apt update && apt install -y git clang curl libssl-dev llvm libudev-dev libc-bin libc6

RUN useradd -m -u 1000 -U -s /bin/sh -d /substrate substrate && \
    mkdir -p /data /substrate/.local/share/substrate && \
    chown -R substrate:substrate /data && \
    ln -s /data /substrate/.local/share/substrate && \
    # unclutter and minimize the attack surface
    rm -rf /usr/bin /usr/sbin && \
    # Sanity checks
    /usr/local/bin/cherry --version

USER substrate
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]
