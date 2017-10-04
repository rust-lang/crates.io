#!/bin/sh

diesel migration run
./script/init-local-index.sh

cargo run --bin server
