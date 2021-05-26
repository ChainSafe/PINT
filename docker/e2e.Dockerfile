# syntax=docker/dockerfile:experimental
#
# Copyright 2021 ChainSafe Systems
# SPDX-License-Identifier: LGPL-3.0-only
#
# This Dockerfile builds the environment of e2e tests
FROM debian:buster-slim
COPY launch launch
COPY --from=chainsafe/pint /usr/local/bin/pint /launch/bin/
COPY --from=clearloop/rococo-v1 /polkadot /launch/bin/
COPY --from=clearloop/statemint /statemint /launch/bin/
ENV CARGO_TERM_COLOR=always
RUN apt-get update -y \
    && apt-get install openssl curl git -y \
    && curl -sL https://deb.nodesource.com/setup_15.x | bash - \
    && apt-get -qqy --no-install-recommends install nodejs -y \
    && rm -f /var/cache/apt/archives/*.deb /var/cache/apt/archives/partial/*.deb \
    && rm -f /var/cache/apt/*.bin \
    && git clone https://github.com/paritytech/polkadot-launch.git \
    && cd polkadot-launch \
    && npm install \
    && npm run build
EXPOSE 9966
EXPOSE 9988
EXPOSE 9999
ENTRYPOINT [ "node", "polkadot-launch/dist/index.js", "launch/config.json" ]
