release: bin/diesel migration run
web: bin/start-nginx npm run nf -- --procfile foreman-procfile start --raw
background_worker: ./target/release/background-worker
