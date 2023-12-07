# renovate: datasource=github-tags depName=rust lookupName=rust-lang/rust
ARG RUST_VERSION=1.74.1@sha256:32d220ca8c77fe56afd6d057c382ea39aced503278526a34fc62b90946f92e02

FROM rust:$RUST_VERSION

# renovate: datasource=crate depName=diesel_cli versioning=semver
ARG DIESEL_CLI_VERSION=2.1.1

RUN apt-get update \
    && apt-get install -y postgresql \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install diesel_cli --version $DIESEL_CLI_VERSION --no-default-features --features postgres

WORKDIR /app
COPY . /app

ENTRYPOINT ["/app/docker_entrypoint.sh"]
