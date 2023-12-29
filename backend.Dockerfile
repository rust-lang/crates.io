# renovate: datasource=github-tags depName=rust lookupName=rust-lang/rust
ARG RUST_VERSION=1.75.0@sha256:e17a45360b8569720da89dd7bf3c8628ba801c0758c8b4f12b1b32a9327a43a7

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
