#!/bin/sh

# If the backend is started before postgres is ready, the migrations will fail
until diesel migration run --locked-schema; do
  echo "Migrations failed, retrying in 5 seconds..."
  sleep 5
done

./script/init-local-index.sh

cargo run --bin server
