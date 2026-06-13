#!/usr/bin/env bash
# heimq stress test via kcat.
# Parameterized: TOPICS, MSGS_PER_TOPIC, PRODUCERS (parallel), CONSUMERS (parallel).
# Produces MSGS_PER_TOPIC * PRODUCERS messages to each topic, across TOPICS topics,
# using varying partition counts. Verifies total consumed == total produced.

set -uo pipefail

TOPICS="${TOPICS:-3}"
MSGS_PER_TOPIC="${MSGS_PER_TOPIC:-100}"
PRODUCERS="${PRODUCERS:-1}"        # parallel producers per topic
CONSUMERS="${CONSUMERS:-1}"        # parallel consumers per group
GROUPS_PER_TOPIC="${GROUPS_PER_TOPIC:-1}"  # independent groups per topic
BROKER="127.0.0.1:9092"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

KCAT="${KCAT:-kcat}"
command -v "$KCAT" >/dev/null || { echo "error: $KCAT not found"; exit 1; }

PARTITION_COUNTS=(1 2 3 5 8 12 16 24 32 64)

# Build topic specs for HEIMQ_CREATE_TOPICS env var.
TOPIC_NAMES=()
TOPIC_SPECS=()
for i in $(seq 1 "$TOPICS"); do
  TOPIC_NAMES+=("stress-topic-$i")
  pc_idx=$(( (i - 1) % ${#PARTITION_COUNTS[@]} ))
  TOPIC_SPECS+=("stress-topic-$i:${PARTITION_COUNTS[$pc_idx]}")
done
TOPIC_SPECS_CSV=$(IFS=,; echo "${TOPIC_SPECS[*]}")

FAILED=0
pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1"; FAILED=1; }

echo "==> params: TOPICS=$TOPICS MSGS_PER_TOPIC=$MSGS_PER_TOPIC PRODUCERS=$PRODUCERS CONSUMERS=$CONSUMERS"
echo "==> total target: $((TOPICS * MSGS_PER_TOPIC * PRODUCERS)) messages"

echo "==> building heimq (release)"
( cd "$REPO_DIR" && cargo build --release --quiet ) || { echo "build failed"; exit 1; }

echo "==> starting heimq with pre-created topics: $TOPIC_SPECS_CSV"
HEIMQ_CREATE_TOPICS="$TOPIC_SPECS_CSV" \
"$REPO_DIR/target/release/heimq" \
  --host 127.0.0.1 --port 9092 --default-partitions 3 \
  >/tmp/heimq-stress.log 2>&1 &
HEIMQ_PID=$!
cleanup() {
  kill $HEIMQ_PID 2>/dev/null || true
  if [ "$FAILED" = "0" ]; then rm -rf /tmp/kcat-stress-work; fi
}
trap cleanup EXIT

for _ in $(seq 1 50); do
  if "$KCAT" -b "$BROKER" -L -m 1 >/dev/null 2>&1; then break; fi
  sleep 0.2
done

WORK=/tmp/kcat-stress-work
rm -rf "$WORK"; mkdir -p "$WORK"

# Create topics up-front via produce-one (auto-create with default_partitions).
# To get *varied* partition counts, we need explicit CreateTopics. kcat
# doesn't issue that directly, so we use python-kafka if available, else
# rely on heimq's default_partitions (all topics same count).
#
# Simpler path: restart heimq per topic-group with a different
# default_partitions and use auto-create. But that breaks multi-topic.
# Best path: use a small Rust helper. For now, use default_partitions=3
# for all topics and call out that varied partitions require a different
# mechanism. We log the mismatch if it matters.

echo
echo "==> [produce] $PRODUCERS producers x $TOPICS topics x $MSGS_PER_TOPIC msgs"
PROD_START=$(date +%s)
produce_one() {
  local topic="$1" prod_id="$2" count="$3"
  local out="$WORK/prod-${topic}-${prod_id}.log"
  # Emit key:value lines. Payload size rotates: small/medium/large when
  # VARY_PAYLOAD=1, so record batches exercise varying byte widths.
  local vary="${VARY_PAYLOAD:-0}"
  awk -v p="$prod_id" -v c="$count" -v vary="$vary" '
    function rep(s, n,   out, i){ out=""; for(i=0;i<n;i++) out=out s; return out }
    BEGIN{
      small = "s"
      medium = rep("m", 1024)
      large = rep("l", 16384)
      for (i=0; i<c; i++) {
        if (vary == "1") {
          mod = i % 3
          if (mod == 0) body = small "-" i
          else if (mod == 1) body = medium "-" i
          else body = large "-" i
        } else {
          body = "p" p "-msg" i
        }
        printf "prod%d-key%d:%s\n", p, i, body
      }
    }' \
    | "$KCAT" -b "$BROKER" -t "$topic" -P -K: 2>"$out"
}
export -f produce_one
export KCAT BROKER WORK VARY_PAYLOAD

pids=()
for topic in "${TOPIC_NAMES[@]}"; do
  for pid_i in $(seq 1 "$PRODUCERS"); do
    produce_one "$topic" "$pid_i" "$MSGS_PER_TOPIC" &
    pids+=($!)
  done
done

for p in "${pids[@]}"; do wait "$p" || fail "producer pid $p exited nonzero"; done
PROD_END=$(date +%s)
echo "  produce elapsed: $((PROD_END - PROD_START))s"

# Let broker settle
sleep 1

echo
echo "==> [consume] verify counts per topic"
TOTAL_EXPECTED_PER_TOPIC=$((MSGS_PER_TOPIC * PRODUCERS))
GRAND_EXPECTED=$((TOTAL_EXPECTED_PER_TOPIC * TOPICS))
GRAND_CONSUMED=0

for topic in "${TOPIC_NAMES[@]}"; do
  CONSUMED=$("$KCAT" -b "$BROKER" -t "$topic" -C -o beginning -e \
    -f '%s\n' 2>"$WORK/cons-$topic.err" | wc -l | tr -d ' ')
  if [ "$CONSUMED" = "$TOTAL_EXPECTED_PER_TOPIC" ]; then
    pass "$topic: $CONSUMED/$TOTAL_EXPECTED_PER_TOPIC"
  else
    fail "$topic: $CONSUMED/$TOTAL_EXPECTED_PER_TOPIC (stderr: $(head -3 "$WORK/cons-$topic.err" | tr '\n' ';'))"
  fi
  GRAND_CONSUMED=$((GRAND_CONSUMED + CONSUMED))
done

echo
echo "==> [consume-group] $GROUPS_PER_TOPIC groups x $CONSUMERS consumers per topic"
if [ "$CONSUMERS" -gt 0 ] && [ "$GROUPS_PER_TOPIC" -gt 0 ]; then
  group_start=$(date +%s)
  expected_keys=$((MSGS_PER_TOPIC * PRODUCERS))
  for topic in "${TOPIC_NAMES[@]}"; do
    allpids=()
    # Launch all groups for this topic in parallel; each group is
    # independent so they should not interfere.
    for g_i in $(seq 1 "$GROUPS_PER_TOPIC"); do
      group="stress-group-${topic}-g${g_i}"
      ( timeout 60 "$KCAT" -b "$BROKER" -G "$group" "$topic" -e \
          -f '%k\n' -o beginning 2>"$WORK/grp-${topic}-g${g_i}-1.err" \
          > "$WORK/grp-${topic}-g${g_i}-1.out" || true ) &
      allpids+=($!)
      for c_i in $(seq 2 "$CONSUMERS"); do
        sleep 0.2
        ( timeout 60 "$KCAT" -b "$BROKER" -G "$group" "$topic" -e \
            -f '%k\n' -o beginning 2>"$WORK/grp-${topic}-g${g_i}-${c_i}.err" \
            > "$WORK/grp-${topic}-g${g_i}-${c_i}.out" || true ) &
        allpids+=($!)
      done
    done
    for p in "${allpids[@]}"; do wait "$p" || true; done

    # Verify each group independently saw >= expected_keys unique keys
    for g_i in $(seq 1 "$GROUPS_PER_TOPIC"); do
      uniq_count=$(cat "$WORK/grp-${topic}-g${g_i}"-*.out 2>/dev/null \
        | grep -v '^$' | sort -u | wc -l | tr -d ' ')
      if [ "$uniq_count" -ge "$expected_keys" ]; then
        pass "group g$g_i/$topic: $uniq_count unique keys (>= $expected_keys)"
      else
        fail "group g$g_i/$topic: only $uniq_count (< $expected_keys)"
      fi
    done
  done
  group_end=$(date +%s)
  echo "  consumer group elapsed: $((group_end - group_start))s"
fi

echo
echo "==> summary"
echo "  expected total: $GRAND_EXPECTED"
echo "  consumed total: $GRAND_CONSUMED"

if [ "$FAILED" = "0" ] && [ "$GRAND_CONSUMED" = "$GRAND_EXPECTED" ]; then
  echo "==> ALL CHECKS PASSED"
  exit 0
else
  echo "==> ONE OR MORE CHECKS FAILED (see /tmp/heimq-stress.log and $WORK/)"
  exit 1
fi
