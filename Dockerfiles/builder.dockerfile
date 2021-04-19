FROM rust:1 as builder

COPY . .

ARG RUST_TOOLCHAIN=nightly-2020-11-25
ENV CARGO_TERM_COLOR=always

RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev cmake \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup target add wasm32-unknown-unknown \
    && cargo build --release
