#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
# CONTRIBUTING.md の CLA の節（`## CLA` から `## 送る前に` まで）が、基準の版（既定 origin/main）から変わっていないことを見る。
# ST53 Task 1.1 の検証（書き足すのは冒頭の許諾の文だけで、CLA の中身は変えない）。`## 送る前に` の手元の検査の行は別の Task が直す
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
base="${1:-origin/main}"
section() { sed -n '/^## CLA/,/^## 送る前に/p'; }
expected="$(git -C "$root" show "$base:CONTRIBUTING.md" | section)"
actual="$(section < "$root/CONTRIBUTING.md")"
[[ -n "$expected" ]] || { echo "NG: $base の CONTRIBUTING.md に CLA の節が無い" >&2; exit 1; }
if ! diff <(printf '%s\n' "$expected") <(printf '%s\n' "$actual"); then
  echo "NG: CONTRIBUTING.md の CLA の節が $base と違う" >&2
  exit 1
fi
echo "OK: CONTRIBUTING.md の CLA の節は $base と同じ"
