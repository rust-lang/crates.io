#! /bin/bash
set -ue

if [[ "${USE_FASTBOOT:-0}" = 1 ]]; then
    export USE_FASTBOOT=1
    node --optimize_for_size --max_old_space_size=200 fastboot.js &
    bin/start-nginx ./target/release/server &
    wait -n
else
    unset USE_FASTBOOT
    bin/start-nginx ./target/release/server
fi
