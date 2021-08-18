#!/usr/bin/env bash

set -e

echo "fetching updates from production app…"
git fetch heroku-prod

prod_sha=`git rev-parse --short heroku-prod/master`
master_sha=`git rev-parse --short master`

echo ""
echo "production app is at: ${prod_sha}"
echo "local \`master\` branch is at: ${master_sha}"
echo ""
echo "https://github.com/rust-lang/crates.io/compare/${prod_sha}...${master_sha}"
