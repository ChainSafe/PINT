# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# Builder layer
FROM paritytech/ci-linux:production as builder
COPY . .
ENV CARGO_TERM_COLOR=always
RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=target \
    cargo build --release \
    && mv target/release/pint /pint


# Release Image
FROM debian:buster-slim
COPY --from=builder /pint /usr/local/bin/

# 30333 for p2p traffic
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 9933 9944 9615
ENTRYPOINT [ "/usr/local/bin/pint" ]
