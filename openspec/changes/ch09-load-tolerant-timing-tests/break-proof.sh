#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
[ "${1:-}" = history ] || { echo "使い方: break-proof.sh history" >&2; exit 2; }

echo "Scenario: 見回りが履歴の取得を待つように壊すと、見張りの上限で試験が落ちる"
cd "$ROOT"
tools/break-check.sh \
  --patch openspec/changes/ch09-load-tolerant-timing-tests/break-wait-on-read.patch \
  --expect 見張りが読みを放した \
  --timeout 900 -- \
  cargo test -p ashiato-collector-windows --lib history_large_fetch_keeps_observing_foreground
