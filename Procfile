web: bin/diesel migration run --locked-schema && bin/start-nginx ./target/release/server
worker: ./target/release/update-downloads daemon 300
