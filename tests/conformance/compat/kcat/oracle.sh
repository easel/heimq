#!/bin/sh
# kcat compatibility oracle. kcat is a CLI over librdkafka; this exercises
# heimq's Produce, offset Fetch, Metadata, and consumer-group paths through it.
#
# Ported from crates/heimq/tests/compat.rs::test_kcat_produce_consume_roundtrip.
set -eu
BOOTSTRAP="${1:?usage: oracle.sh <host:port>}"
TS="$(date +%s)-$$"  # busybox date has no %N
K() { kcat -b "$BOOTSTRAP" "$@"; }

fail() { echo "FAIL: $*" >&2; exit 1; }

# 1. Plain produce, then offset-based consume (no consumer group).
TOPIC="kcat-compat-${TS}"
MESSAGES="alpha
beta
gamma
delta
epsilon"
printf '%s' "$MESSAGES" | K -t "$TOPIC" -P
consumed="$(K -t "$TOPIC" -C -o beginning -c 5 -e -q)"
[ "$consumed" = "$MESSAGES" ] || fail "offset-consume mismatch: $consumed"

# 2. Key-value round-trip.
KV_TOPIC="kcat-kv-${TS}"
KV="k1:v1
k2:v2
k3:v3"
printf '%s' "$KV" | K -t "$KV_TOPIC" -P -K :
kv_consumed="$(K -t "$KV_TOPIC" -C -o beginning -c 3 -e -q -f '%k:%s\n')"
[ "$kv_consumed" = "$KV" ] || fail "kv round-trip mismatch: $kv_consumed"

# 3. Metadata listing includes the produced topic.
K -L | grep -q "$TOPIC" || fail "topic $TOPIC absent from kcat -L"

# 4. Consumer group consume drives JoinGroup/SyncGroup/Heartbeat.
CG_TOPIC="kcat-cg-${TS}"
CG_GROUP="kcat-cg-grp-${TS}"
CG="msg-0
msg-1
msg-2
msg-3
msg-4"
printf '%s' "$CG" | K -t "$CG_TOPIC" -P
# Options must precede -G: in group mode kcat treats every later argument as a topic.
cg_consumed="$(K -c 5 -e -q -X auto.offset.reset=earliest -G "$CG_GROUP" "$CG_TOPIC" | grep -v '^$' | sort)"
[ "$cg_consumed" = "$(printf '%s' "$CG" | sort)" ] || fail "consumer-group mismatch: $cg_consumed"

# 5. Consumer group offset resume: consume 3, then a fresh session with the same
#    group must resume at offset 3 rather than replay from the beginning.
R_TOPIC="kcat-resume-${TS}"
R_GROUP="kcat-resume-grp-${TS}"
R="r0
r1
r2
r3
r4
r5"
printf '%s' "$R" | K -t "$R_TOPIC" -P
a="$(K -c 3 -e -q -X auto.offset.reset=earliest -G "$R_GROUP" "$R_TOPIC" | grep -v '^$')"
[ "$a" = "$(printf 'r0\nr1\nr2')" ] || fail "session A expected r0..r2, got: $a"
b="$(K -c 3 -e -q -X auto.offset.reset=earliest -G "$R_GROUP" "$R_TOPIC" | grep -v '^$')"
[ "$b" = "$(printf 'r3\nr4\nr5')" ] || fail "session B did not resume at offset 3, got: $b"

echo "kcat oracle: PASS"
