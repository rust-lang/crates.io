FROM mcr.microsoft.com/devcontainers/rust:2-bookworm@sha256:4d944fc913385b14c0507bc4636749a773543067c2f5fc885b6fd4a42e59f006

# Enable pipefail so a failure anywhere in a pipeline aborts the build.
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

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

# Install the mise version manager (https://mise.jdx.dev) for the `vscode` user.
# The tools themselves are installed by `post-create.sh` via `mise install`.
USER vscode
ENV MISE_DATA_DIR=/home/vscode/.local/share/mise
ENV PNPM_HOME=/home/vscode/.local/share/pnpm
ENV PATH=/home/vscode/.local/bin:$MISE_DATA_DIR/shims:$PATH
RUN curl -fsSL https://mise.run | sh

USER root
