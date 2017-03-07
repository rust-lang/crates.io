web: ./target/release/migrate && diesel migration run && bin/start-nginx ./target/release/server
worker: ./target/release/update-downloads daemon 300
