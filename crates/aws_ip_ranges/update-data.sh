#!/bin/bash
set -euxo pipefail

mkdir -p data

curl https://ip-ranges.amazonaws.com/ip-ranges.json --output data/ip-ranges.json
jq --monochrome-output '.' data/ip-ranges.json > data/ip-ranges.json.tmp
mv data/ip-ranges.json.tmp data/ip-ranges.json
