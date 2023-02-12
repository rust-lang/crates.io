# renovate: datasource=github-releases depName=rust lookupName=rust-lang/rust
ARG RUST_VERSION=1.67.1

FROM rust:$RUST_VERSION

RUN apt-get update \
    && apt-get install -y postgresql \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install diesel_cli --version 1.4.1 --no-default-features --features postgres

WORKDIR /app
COPY . /app

ENTRYPOINT ["/app/docker_entrypoint.sh"]
