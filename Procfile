release: bin/diesel migration run
web: node fastboot.js & bin/start-nginx target/release/server & wait -n
background_worker: ./target/release/background-worker
