#! /bin/bash
set -euo pipefail

./script/init-local-index.sh

# The below are required environment variables
export SESSION_KEY=badkeyabcdefghijklmnopqrstuvwxyzabcdef
export GIT_REPO_CHECKOUT=./tmp/index-co
export GIT_REPO_URL=file://./tmp/index-bare
export GH_CLIENT_ID=
export GH_CLIENT_SECRET=

./target/debug/server > backend.log &
npm run start -- --proxy http://localhost:8888 > frontend.log &

for i in $(seq 1 10)
do
    set +e
    curl -H 'Accept: text/html' http://localhost:4200
	case $? in
	    0)
	        break
	        ;;
	    7)
	        # Connection refused
	        sleep 10
	        ;;
	    56)
	        # Connection reset by peer
	        sleep 10
	        ;;
	    *)
	        exit $?
	esac
    set -e
done

echo FRONTEND
cat frontend.log

echo BACKEND
cat backend.log
