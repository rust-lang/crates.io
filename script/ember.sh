#! /bin/sh
set -ue

export FASTBOOT_DISABLED

if [ "${USE_FASTBOOT:-0}" = '1' ]; then
    unset FASTBOOT_DISABLED
else
    FASTBOOT_DISABLED=1
fi

ember "$@"
