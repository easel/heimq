#!/bin/sh
# Wait for $BOOTSTRAP to accept TCP, then exec the oracle command as given.
#
# There is no availability check and no skip path: every runtime lives in this
# image, so an oracle that cannot run is a failure, not a silent pass.
set -e
: "${BOOTSTRAP:?BOOTSTRAP must be set (host:port)}"
host="${BOOTSTRAP%:*}"
port="${BOOTSTRAP##*:}"

i=0
until nc -z "$host" "$port" 2>/dev/null; do
  i=$((i + 1))
  if [ "$i" -ge 120 ]; then
    echo "FAIL: $BOOTSTRAP unreachable after 60s" >&2
    exit 1
  fi
  sleep 0.5
done

exec "$@"
