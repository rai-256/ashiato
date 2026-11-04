#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
[ "${1:-}" = history ] || [ "${1:-}" = scan ] || {
  echo "使い方: break-proof.sh history|scan" >&2
  exit 2
}

cd "$ROOT"
if [ "$1" = history ]; then
  echo "Scenario: 見回りが履歴の取得を待つように壊すと、見張りの上限で試験が落ちる"
  tools/break-check.sh \
    --patch openspec/changes/ch09-load-tolerant-timing-tests/break-wait-on-read.patch \
    --expect 見張りが読みを放した \
    --timeout 900 -- \
    cargo test -p ashiato-collector-windows --lib history_large_fetch_keeps_observing_foreground
else
  set -a
  [ -f .env ] && . ./.env
  set +a
  . tools/ports.sh
  scan_target=$(mktemp -d "${TMPDIR:-/tmp}/ashiato-break-scan-target.XXXXXX")
  trap 'rm -rf -- "$scan_target"' EXIT
  echo "Scenario: 取り込み器が変えた間隔を読まないように壊すと、間隔の変更の試験は期限で落ちる"
  CARGO_TARGET_DIR="$scan_target" tools/break-check.sh \
    --patch openspec/changes/ch09-load-tolerant-timing-tests/break-ignore-interval.patch \
    --expect 待った時間 \
    --expect 最後に見た数 \
    --timeout 900 -- \
    cargo test -p ashiato-server archive_scan_interval_change_takes_effect_without_restart
fi
