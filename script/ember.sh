#! /bin/sh
set -ue

export FASTBOOT_DISABLED

if [ -z "${USE_FASTBOOT-}" ]; then
    FASTBOOT_DISABLED=1
else
    unset FASTBOOT_DISABLED
fi

ember "$@"
