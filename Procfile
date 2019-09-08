release: bin/diesel migration run
web: node --optimize_for_size --max_old_space_size=200 fastboot.js & bin/start-nginx target/release/server & wait -n
background_worker: ./target/release/background-worker
