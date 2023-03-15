#!/usr/bin/env bash

#set -x
readonly sig=${1:?signal required}
readonly mode=${2:-debug}

args=(
    --errlog-level debug
    -e 'print("SLEEP START")'
    -e 'ngx.sleep(5)'
    -e 'print("SLEEP END")'
)

cargo build &>/dev/null
cargo build --release &>/dev/null

PID=

case $mode in
    resty)
        ./resty.pl "${args[@]}" &
        PID=$!
        ;;
    debug)
        cargo run --quiet -- "${args[@]}" &
        PID=$!
        ;;
    release)
        cargo run --quiet --release -- "${args[@]}" &
        PID=$!
        ;;
    *)
        echo WTF?
        exit 1
esac


#strace -s 512 -t -f -p "$pid" -o trace.txt &
#strace_pid=$!

sleep 1

echo "sending sig $sig to $PID"
kill -${sig} "$PID"


wait "$PID"

echo "Exit code: $?"
#kill "$strace_pid"
