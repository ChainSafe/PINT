# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# Builder layer
FROM paritytech/ci-linux:production as builder

WORKDIR /pint
COPY . .

ENV CARGO_TERM_COLOR=always

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/pint/target \
    cargo build --release -vv

# Release Image
FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /home/rust/target/release/pint /usr/local/bin/

# 30333 for p2p traffic
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 9933 9944 9615

ENTRYPOINT [ "/usr/local/bin/pint" ]
