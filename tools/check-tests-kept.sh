#!/usr/bin/env bash
# 試験の印と名前が base から減っていないか（CH04 の C1。Task 10.2）。
#   tools/check-tests-kept.sh [<base>] [<path>]   既定は origin/main と crates/server
# 消えた `Scenario:` の行は、足した行にも一字一句同じものがあること。
# 消えた `fn <名前>` は、足した行にも同じ名前があること —— 行で比べると、補助関数の引数を変えただけで
# 「消えた」と見てしまい、行頭から始まる `fn` も拾えなかった（2026-10-25 CH04 Q5）。
set -euo pipefail
cd "$(dirname "$0")/.."
base=${1:-origin/main}
path=${2:-crates/server}
d=$(git diff "$base"...HEAD -- "$path" 2>/dev/null)

removed() { printf '%s\n' "$d" | grep -E "^-([^-].*)?$1" | sed 's/^-//' || true; }
added() { printf '%s\n' "$d" | grep -E "^\+([^+].*)?$1" | sed 's/^+//' || true; }
names() { grep -oE 'fn [A-Za-z0-9_]+' || true; }

lost_scn=$(comm -23 <(removed 'Scenario:' | sort -u) <(added 'Scenario:' | sort -u) | grep -v '^$' || true)
lost_fn=$(comm -23 <(removed 'fn ' | names | sort -u) <(added 'fn ' | names | sort -u) | grep -v '^$' || true)

rc=0
if [ -n "$lost_scn" ]; then
  echo "[FAIL] 消えた Scenario の印:"; printf '  %s\n' "$lost_scn"; rc=1
fi
if [ -n "$lost_fn" ]; then
  echo "[FAIL] 消えた fn の名前:"; printf '  %s\n' "$lost_fn"; rc=1
fi
[ "$rc" -eq 0 ] && echo "[OK] Scenario の印と fn の名前は $base から減っていない（$path）"
exit "$rc"
