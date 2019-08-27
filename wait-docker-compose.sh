#! /bin/sh

for i in $(seq 1 10)
do
    curl -H 'Accept: text/html' http://localhost:4200
    case $? in
        0)
            exit
            ;;
        52)
            # Empty reply from server
            sleep 10
            ;;
        56)
            # Connection reset by peer
            sleep 10
            ;;
        *)
            exit $?
    esac
done
