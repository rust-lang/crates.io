#!/bin/sh

set -e

stamp_file=target/rustc_version_stamp
current_version=$(rustc --version)

if [ -f $stamp_file ]; then
    # Compare the current version against the previous version
    if echo "$current_version" | cmp -s $stamp_file -; then
        echo "Version of rustc hasn't changed, keeping the cache intact"
    else
        echo "The version of rustc has changed, running cargo clean"
        cargo clean
    fi
else
    echo "There is no existing version stamp, keeping the cache intact"
fi

# Save the version stamp for next time
mkdir -p target/
echo $current_version > $stamp_file
