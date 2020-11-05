#!/bin/bash

set -e

if [[ ! -d "target/debug" ]]; then
    echo "No build artifacts found. Exit immediately."
    exit 0
fi

echo "Initial cache size:"
du -hs target/debug

crate_name="cargo-registry"
test_name="all"
bin_names="background-worker crates-admin delete-crate delete-version enqueue-job monitor populate render-readmes server test-pagerduty transfer-crates verify-token"

normalized_crate_name=${crate_name//-/_}
rm -vf target/debug/$normalized_crate_name-*
rm -vf target/debug/deps/$normalized_crate_name-*
rm -vf target/debug/deps/lib$normalized_crate_name-*

normalized_test_name=${test_name//-/_}
rm -vf target/debug/$normalized_test_name-*
rm -vf target/debug/deps/$normalized_test_name-*

for name in $bin_names; do
    rm -vf target/debug/$name
    normalized=${name//-/_}
    rm -vf target/debug/$normalized-*
    rm -vf target/debug/deps/$normalized-*
done

rm -vf target/.rustc_info.json

echo "Final cache size:"
du -hs target/debug
