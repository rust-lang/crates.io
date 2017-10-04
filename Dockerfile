FROM rust:1.20-stretch

RUN apt-get update \
    && apt-get install -y postgresql cmake \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install diesel_cli --no-default-features --features postgres

WORKDIR /app
COPY . /app

ENTRYPOINT ["/app/docker_entrypoint.sh"]
