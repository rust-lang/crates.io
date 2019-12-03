#! /bin/sh
set -ue

export FASTBOOT_DISABLED

if [ -z "${USE_FASTBOOT}" ]; then
    unset FASTBOOT_DISABLED
else
    FASTBOOT_DISABLED=1
fi

ember "$@"
