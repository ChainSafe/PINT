# syntax=docker/dockerfile:experimental
#
# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# This Dockerfile builds the environment of e2e tests
FROM debian:buster-slim
COPY config.json config.json
COPY --from=chainsafe/pint /usr/local/bin/pint bin/
COPY --from=clearloop/rococo-v1 /polkadot bin/
COPY --from=clearloop/statemint /statemint bin/
ENV CARGO_TERM_COLOR=always
RUN apt-get update -y \
    && apt-get install openssl curl git -y \
    && curl -sL https://deb.nodesource.com/setup_15.x | bash - \
    && apt-get -qqy --no-install-recommends install nodejs -y \
    && rm -f /var/cache/apt/archives/*.deb /var/cache/apt/archives/partial/*.deb \
    && rm -f /var/cache/apt/*.bin \
    && git clone https://github.com/paritytech/polkadot-launch.git --depth=1 \
    && cd polkadot-launch \
    && npm install \
    && npm run build
EXPOSE 9988
ENTRYPOINT [ "node", "polkadot-launch/dist/index.js", "config.json" ]
