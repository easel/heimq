#!/usr/bin/env bash
# heimq backend matrix: run kcat-stress.sh against each sensible
# combination of storage backends (log / offsets / groups).
#
# Combinations:
#   1. log=memory, offsets=memory,   groups=memory  (default)
#   2. log=memory, offsets=postgres, groups=memory  (durability slice)
#
# The postgres row is skipped (with a SKIPPED line, not a FAIL) when
# POSTGRES_URL is unset, so the script remains green in environments
# that have no Postgres available.
#
# Volume defaults to 5 topics x 10k msgs and can be overridden via the
# usual kcat-stress.sh env vars (TOPICS, MSGS_PER_TOPIC, etc.).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STRESS="$SCRIPT_DIR/kcat-stress.sh"

[ -x "$STRESS" ] || { echo "error: $STRESS not found or not executable"; exit 2; }

# Moderate volume per the bead: 5 topics x 10k msgs.
export TOPICS="${TOPICS:-5}"
export MSGS_PER_TOPIC="${MSGS_PER_TOPIC:-10000}"
export PRODUCERS="${PRODUCERS:-1}"
export CONSUMERS="${CONSUMERS:-1}"
export GROUPS_PER_TOPIC="${GROUPS_PER_TOPIC:-1}"

# Matrix rows: "label|log|offsets|groups|requires_pg"
ROWS=(
  "memory-only|memory://|memory://|memory://|0"
  "pg-offsets|memory://|__POSTGRES__|memory://|1"
)

PG_URL="${POSTGRES_URL:-}"

PASS_ROWS=()
FAIL_ROWS=()
SKIP_ROWS=()

run_row() {
  local label="$1" log="$2" offsets="$3" groups="$4" requires_pg="$5"

  if [ "$requires_pg" = "1" ]; then
    if [ -z "$PG_URL" ]; then
      echo
      echo "######################################################################"
      echo "# SKIPPED row: $label (POSTGRES_URL is unset)"
      echo "######################################################################"
      SKIP_ROWS+=("$label")
      return 0
    fi
    if [ "$offsets" = "__POSTGRES__" ]; then offsets="$PG_URL"; fi
    if [ "$log" = "__POSTGRES__" ]; then log="$PG_URL"; fi
    if [ "$groups" = "__POSTGRES__" ]; then groups="$PG_URL"; fi
  fi

  echo
  echo "######################################################################"
  echo "# RUN row: $label"
  echo "#   HEIMQ_STORAGE_LOG=$log"
  echo "#   HEIMQ_STORAGE_OFFSETS=$offsets"
  echo "#   HEIMQ_STORAGE_GROUPS=$groups"
  echo "######################################################################"

  if HEIMQ_STORAGE_LOG="$log" \
     HEIMQ_STORAGE_OFFSETS="$offsets" \
     HEIMQ_STORAGE_GROUPS="$groups" \
     "$STRESS"; then
    PASS_ROWS+=("$label")
    return 0
  else
    FAIL_ROWS+=("$label")
    return 1
  fi
}

OVERALL=0
for row in "${ROWS[@]}"; do
  IFS='|' read -r label log offsets groups requires_pg <<<"$row"
  run_row "$label" "$log" "$offsets" "$groups" "$requires_pg" || OVERALL=1
done

echo
echo "==> matrix summary"
for r in "${PASS_ROWS[@]}";  do echo "  PASS:    $r"; done
for r in "${SKIP_ROWS[@]}";  do echo "  SKIPPED: $r"; done
for r in "${FAIL_ROWS[@]}";  do echo "  FAIL:    $r"; done

if [ "$OVERALL" = "0" ]; then
  echo "==> MATRIX OK (${#PASS_ROWS[@]} passed, ${#SKIP_ROWS[@]} skipped)"
  exit 0
else
  echo "==> MATRIX FAILED (${#FAIL_ROWS[@]} failed)"
  exit 1
fi
