#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
printf '%100s\n%100s\n' a b > "$tmp"

out=$(USAGE_VOLUME_SAMPLE_FILE="$tmp" tools/usage-volume.sh)
printf '%s\n' "$out" | grep -q '^events_per_hour=2$'
printf '%s\n' "$out" | grep -q '^bytes_per_event=101$'
printf '%s\n' "$out" | grep -q '^events_per_day=48$'
printf '%s\n' "$out" | grep -q '^bytes_90_days=436320$'

set +e
USAGE_VOLUME_SAMPLE_FILE="$tmp" USAGE_VOLUME_LIMIT_BYTES=1 tools/usage-volume.sh >/dev/null
rc=$?
set -e
[ "$rc" -eq 1 ] || { echo "上限を超えても rc=$rc" >&2; exit 1; }
