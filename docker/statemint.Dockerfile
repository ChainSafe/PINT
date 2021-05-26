# syntax=docker/dockerfile:experimental
#
# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# statemint
FROM paritytech/ci-linux:production as builder
COPY . .
ENV CARGO_TERM_COLOR=always
RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=target \
    git clone https://github.com/paritytech/statemint.git \
    && cd statemint \
    && cargo build --release \
    && mv target/release/statemint /statemint

# Only a binary for debian
FROM scratch
COPY --from=builder /statemint /usr/local/bin/
