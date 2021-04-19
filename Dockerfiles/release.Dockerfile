FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/release/parachain-collator /usr/local/bin/

EXPOSE 30333 9933 9944

ENTRYPOINT [ "/usr/local/bin/parachain-collator" ]
