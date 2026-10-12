# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認のうえで直した記録。

## 2026-10-12 6.1

- 旧: `bash -c 'set -a; . ./.env; set +a; STACK_RESET=1 setsid tools/stack.sh up >/tmp/st25-6.1-stack.log 2>&1 & s=$!; trap "kill -- -$s 2>/dev/null; wait $s 2>/dev/null" EXIT; for i in $(seq 1 1800); do grep -q "^画面:" /tmp/st25-6.1-stack.log && break; kill -0 $s 2>/dev/null || break; sleep 1; done; grep -q "^画面:" /tmp/st25-6.1-stack.log || { tail -20 /tmp/st25-6.1-stack.log; exit 1; }; tools/seed.sh normal && tools/seed.sh normal && curl -sf -H "authorization: Bearer $API_TOKEN" "http://$BIND/day?date=2026-09-07" | jq -e "([.gaps[] | select(.kind==\"away\")] | length) == 3 and ([.gaps[] | select(.kind==\"excluded\")] | length) == 1 and ([.entries[] | select(.summary.pc != null)] | length) >= 1 and ([.entries[] | select(.summary.phone != null)] | length) >= 1"'`
- 新: `tools/st25-seed-check.sh`
- 理由: 旧コマンドは .env の BIND（127.0.0.1:18787）を seed.sh と curl に渡すが、tools/ports.sh（430196c9、ST05）以降 stack.sh は Story の worktree の口（-st25 は 127.0.0.1:19050）でサーバを立てる。18787 には ashiato2-integ のサーバが待ち受けていて、seed の最初の curl -f が 401 で止まる（verify-run 6.1 で rc=22。5180 を ashiato2-integ の確認バッチが持っていたので WEB_PORT=5230 を環境で渡して走らせた）。新しい入口 tools/st25-seed-check.sh は同じ手順で、. tools/ports.sh の後に BIND=127.0.0.1:$ASHIATO_HTTP_PORT を seed.sh と curl の両方に渡す。判定の中身（作り直した DB・seed 2 回・jq -e）は変えない。台本に移したのは plan_fix.py が新しい入口に repo の実行ファイルを求めるため
- 実証: evidence.jsonl の 2026-10-12T11:56:53+00:00（FAIL、rc=22、ログ .superpowers/sdd/logs/20261012-205653-evidence-6.1-0-0d37df.log、HEAD 3238366）
- 承認: deep.md Q9（深掘り第 4 回、deep-answers-3.txt。本人の答え「6.1 の検証コマンドを plan_fix.py で直す」。記録 32383664、2026-10-12）
- 補足（finish）: 新しい入口の初版は `set -euo pipefail` を掛けていて、判定が true でも rc=143 の FAIL になった（evidence.jsonl 2026-10-12T11:57:25+00:00）。原文と同じく `set -e` を外し、seed と curl を `&&` で繋いだ版で PASS（rc=0）。`set -e` があると EXIT の trap の `wait` の非 0 で台本が終わる
