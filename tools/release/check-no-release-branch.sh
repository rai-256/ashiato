#!/usr/bin/env bash
# 修正の枝 release/* の分岐が version.sh に残っていないことを見る（ST61 Task 8.1・D11）。
# 「tools/release/…」のパスは正当なので、枝の名前（release/<数字>）と参照（heads|remotes/origin 上の release/）だけを探す
set -euo pipefail
cd "$(dirname "$0")/../.."
if grep -nE '(refs/heads|refs/remotes/origin|origin)/release/|release/[0-9$]' tools/release/version.sh; then
  echo "version.sh に修正の枝 release/* の分岐が残っている" >&2
  exit 1
fi
echo "修正の枝の分岐は無い"
