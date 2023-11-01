# renovate: datasource=github-tags depName=rust lookupName=rust-lang/rust
ARG RUST_VERSION=1.73.0@sha256:0dc5fcb189be90ebbee985925219f888c257d90f3870b1e7c987e46b0acb5c5e

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
