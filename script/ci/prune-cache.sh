#!/bin/bash

set -e

echo "Initial cache size:"
du -hs target/debug

crate_name="cargo-registry"
test_name="all"
bin_names="delete-crate delete-version populate render-readmes server test-pagerduty transfer-crates update-downloads background-worker monitor"

normalized_crate_name=${crate_name//-/_}
rm -v target/debug/$normalized_crate_name-*
rm -v target/debug/deps/$normalized_crate_name-*
rm -v target/debug/deps/lib$normalized_crate_name-*

normalized_test_name=${test_name//-/_}
rm -v target/debug/$normalized_test_name-*
rm -v target/debug/deps/$normalized_test_name-*

for name in $bin_names; do
    rm -v target/debug/$name
    normalized=${name//-/_}
    rm -v target/debug/$normalized-*
    rm -v target/debug/deps/$normalized-*
done

echo "Final cache size:"
du -hs target/debug
