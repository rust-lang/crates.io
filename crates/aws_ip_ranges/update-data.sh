#!/bin/bash
set -euxo pipefail

mkdir -p data

curl https://ip-ranges.amazonaws.com/ip-ranges.json --output data/ip-ranges.json.full

# Filter to only CloudFront IP ranges
jq --monochrome-output '{
  syncToken: .syncToken,
  createDate: .createDate,
  prefixes: [.prefixes[] | select(.service == "CLOUDFRONT")],
  ipv6_prefixes: [.ipv6_prefixes[] | select(.service == "CLOUDFRONT")]
}' data/ip-ranges.json.full > data/ip-ranges.json

rm data/ip-ranges.json.full
