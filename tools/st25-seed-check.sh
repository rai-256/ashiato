#!/usr/bin/env bash
# ST25 tasks 6.1 の検証: 作り直した DB に `seed.sh normal` を 2 回入れても畳まれ、2026-09-07 の `/day` が
# 離席 3・除外 1・PC とスマホの要約を持つ。
#
# もとは tasks.md に書いた `bash -c '…'` 1 行で、`.env` の BIND（127.0.0.1:18787）を seed と curl に渡していた。
# tools/ports.sh（ST05）以降 stack.sh は Story の worktree の口でサーバを立てるので、seed と curl にも同じ口を渡す
# （deep.md Q9。plan_fix.py は新しい入口に repo の実行ファイルを求めるので、台本に移した。判定の中身は変えていない）。
# 原文と同じく set -e は掛けない。掛けると、EXIT の trap の中で `wait` が止めた stack.sh の 143 を返し、errexit で台本が非 0 で終わる
# （2026-10-12 実測: 判定が true でも rc=143）
cd "$(dirname "$0")/.." || exit 1
set -a; . ./.env; . tools/ports.sh; BIND=127.0.0.1:$ASHIATO_HTTP_PORT; set +a
STACK_RESET=1 setsid tools/stack.sh up >/tmp/st25-6.1-stack.log 2>&1 & s=$!
trap "kill -- -$s 2>/dev/null; wait $s 2>/dev/null" EXIT
for i in $(seq 1 1800); do grep -q "^画面:" /tmp/st25-6.1-stack.log && break; kill -0 $s 2>/dev/null || break; sleep 1; done
grep -q "^画面:" /tmp/st25-6.1-stack.log || { tail -20 /tmp/st25-6.1-stack.log; exit 1; }
tools/seed.sh normal && tools/seed.sh normal && curl -sf -H "authorization: Bearer $API_TOKEN" "http://$BIND/day?date=2026-09-07" | jq -e "([.gaps[] | select(.kind==\"away\")] | length) == 3 and ([.gaps[] | select(.kind==\"excluded\")] | length) == 1 and ([.entries[] | select(.summary.pc != null)] | length) >= 1 and ([.entries[] | select(.summary.phone != null)] | length) >= 1"
