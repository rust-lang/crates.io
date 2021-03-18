#! /bin/bash
set -ue

# Since this script is launched from our app, we tell the nginx
# buildpack (`bin/start-nginx`) that `cat` is our server.

if [[ -z "${USE_FASTBOOT-}" ]]; then
    unset USE_FASTBOOT
    bin/start-nginx cat
else
    export USE_FASTBOOT
    node --optimize_for_size --max_old_space_size=200 fastboot.js &
    bin/start-nginx cat &
    wait -n
fi
