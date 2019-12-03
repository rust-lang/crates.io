#! /bin/bash
set -ue

if [[ -z "${USE_FASTBOOT-}" ]]; then
    unset USE_FASTBOOT
    bin/start-nginx ./target/release/server
else
    export USE_FASTBOOT
    node --optimize_for_size --max_old_space_size=200 fastboot.js &
    bin/start-nginx ./target/release/server &
    wait -n
fi
