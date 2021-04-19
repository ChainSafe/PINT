# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# Builder layer
FROM rust as builder

COPY . .

ARG RUST_TOOLCHAIN=nightly-2020-11-25
ENV CARGO_TERM_COLOR=always

RUN --mount=type=cache,target=/home/rust/.cargo/git \
    --mount=type=cache,target=/home/rust/.cargo/registry \
    --mount=type=cache,sharing=private,target=/home/rust/src/target \
    apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev cmake \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup target add wasm32-unknown-unknown \
    && cargo build --release

# Release Image
FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /home/rust/target/release/pint /usr/local/bin/

EXPOSE 30333 9933 9944

ENTRYPOINT [ "/usr/local/bin/parachain-collator" ]
