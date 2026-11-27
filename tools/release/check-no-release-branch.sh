#!/usr/bin/env bash
# 修正の枝 release/* の分岐が version.sh に残っていないことを見る（ST61 Task 8.1・D11）。
# 試験（number_self_test より後）は旧版の枝の名前を GIVEN に持つので見ない（plan() などの本体だけを走査する）
# 「tools/release/…」のパスは正当なので、枝の名前（release/<数字>）と参照（heads|remotes/origin 上の release/）だけを探す
set -euo pipefail
cd "$(dirname "$0")/../.."
body=$(awk '/^number_self_test\(\) \{/{exit} {print NR":"$0}' tools/release/version.sh)
if grep -E '(refs/heads|refs/remotes/origin|origin)/release/|release/[0-9$]' <<<"$body"; then
  echo "version.sh に修正の枝 release/* の分岐が残っている" >&2
  exit 1
fi
echo "修正の枝の分岐は無い"
