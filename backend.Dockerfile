# renovate: datasource=github-tags depName=rust lookupName=rust-lang/rust
ARG RUST_VERSION=1.76.0@sha256:e21cbd18f8bafcf552b78ef62b4b2ccd5cbbc9df739d26ee1836d46fb2591d2f

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
