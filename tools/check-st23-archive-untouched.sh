#!/usr/bin/env bash
# ST23 が書庫の取り込み器（crates/server/src/archive/worker.rs）を変えていないことを確かめる（design D9（仮）・tasks 6.1）。
# 比べる相手は main の先端でなく、このブランチが main から分かれた点（main 側の変化はこのブランチの変更ではない）。
# 作業ツリーの未 commit の変更も含めて比べる。
set -euo pipefail
cd "$(dirname "$0")/.."
base="$(git merge-base origin/main HEAD)"
if git diff --quiet "$base" -- crates/server/src/archive/worker.rs; then
  echo "OK archive/worker.rs は分岐点（${base:0:8}）から変わっていない"
else
  git diff --stat "$base" -- crates/server/src/archive/worker.rs
  echo "NG archive/worker.rs を変えている（分岐点 ${base:0:8} との差）"
  exit 1
fi
