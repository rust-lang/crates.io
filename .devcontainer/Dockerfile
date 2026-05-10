FROM mcr.microsoft.com/devcontainers/rust:2-bookworm@sha256:9cf2686b6f111f9c1eef20b8103a5f15e96c5a94e2d4d8efeed7af4453872a93

# Add the official PostgreSQL apt repo so we can install PostgreSQL 16
# client + libpq. Debian bookworm only ships v15.
RUN install -d /usr/share/postgresql-common/pgdg \
    && curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
       -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc \
    && echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
       > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        postgresql-client-16 \
        libpq-dev \
        git \
    && rm -rf /var/lib/apt/lists/*

# renovate: datasource=crate depName=diesel_cli versioning=semver
ARG DIESEL_CLI_VERSION=2.3.9

RUN cargo install --locked diesel_cli --version $DIESEL_CLI_VERSION \
    --no-default-features --features postgres

# Install pnpm under /home/vscode (its $PNPM_HOME) so the named pnpm
# store volume can be mounted there. Switch back to root afterwards
# so any downstream Feature install scripts run as expected.
USER vscode
ENV PNPM_HOME=/home/vscode/.local/share/pnpm
ENV PATH=$PNPM_HOME:$PATH
RUN curl -fsSL https://get.pnpm.io/install.sh | env SHELL=/bin/bash sh -

USER root
