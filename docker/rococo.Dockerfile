# syntax=docker/dockerfile:experimental
#
# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# rococo-v1
FROM paritytech/ci-linux:staging-1.55.0-stable as builder
COPY . .
ENV CARGO_TERM_COLOR=always
RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=target \
    git clone https://github.com/paritytech/polkadot.git -b rococo-v1 --depth=1 \
    && cd polkadot \
    && cargo build --release \
    && mv target/release/polkadot /polkadot

# Only a binary for debian
FROM scratch
COPY --from=builder /polkadot /
