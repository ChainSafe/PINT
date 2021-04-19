# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# Builder layer
FROM rust as builder

WORKDIR /pint
COPY . .

ENV CARGO_TERM_COLOR=always

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/pint/target \
    apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev cmake \
    && rustup default nightly-2020-11-25 \
    && rustup target add wasm32-unknown-unknown \
    && cargo build --release -vv

# Release Image
FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /home/rust/target/release/pint /usr/local/bin/

EXPOSE 30333 9933 9944

ENTRYPOINT [ "/usr/local/bin/pint" ]
