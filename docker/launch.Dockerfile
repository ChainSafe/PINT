# syntax=docker/dockerfile:experimental
#
# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# This Dockerfile builds the environment of e2e tests
FROM debian:buster-slim
COPY config.json config.json
COPY js/polkadot-launch polkadot-launch
COPY --from=chainsafe/pint /usr/local/bin/pint bin/
COPY --from=parity/polkadot:v0.9.12 /usr/bin/polkadot bin/
ENV CARGO_TERM_COLOR=always
RUN apt-get update -y \
    && apt-get install openssl curl git -y \
    && curl -sL https://deb.nodesource.com/setup_15.x | bash - \
    && apt-get -qqy --no-install-recommends install nodejs -y \
    && rm -f /var/cache/apt/archives/*.deb /var/cache/apt/archives/partial/*.deb \
    && rm -f /var/cache/apt/*.bin \
    && cd polkadot-launch \
    && npm install \
    && npm run build
EXPOSE 9988
ENTRYPOINT [ "node", "polkadot-launch/dist/cli.js", "config.json" ]
