#! /bin/bash
set -ue

# Since this script is launched from our app, we tell the nginx
# buildpack (`bin/start-nginx`) that `cat` is our server.

bin/start-nginx cat
