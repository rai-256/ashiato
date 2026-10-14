# plan の訂正（凍結後の検証コマンドだけ）

受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認のうえで直した記録。

## 2026-10-14 9.3

- 旧: cargo test -p ashiato-collector-windows --test runtime_windows outage -- --test-threads=1（WSL で走る）
- 新: WSL から rsync で C:\dev\ashiato2-rt-st14 へ写し、cmd.exe から Windows 側の cargo で同じ試験を走らせる（ST08 の 10.1 / 10.2 と同じ形）
- 理由: runtime_windows.rs は Windows でしかコンパイルされない。WSL では rc=0 でも試験 0 本で FAIL（fp=e49719f9f0、2 回）
- 手で直した理由: 新しい入口が `bash -c` で、plan_fix.py の「最初の語が repo の実行ファイル」を満たさない
- 承認: 2026-10-14 本人（会話で「st14 は直して良い」）

## 2026-10-14 1.1

- 旧: bash -c 'grep -q "DEVICE_SUBJECT" crates/server/src/coverage.rs && ! grep -rq "outage_target" migrations/ && grep -q "archive_ledger_source" migrations/202609181600_archive_ingestion.sql'
- 新: bash -c 'grep -q "DEVICE_SUBJECT" crates/server/src/coverage.rs && grep -q "archive_ledger_source" migrations/202609181600_archive_ingestion.sql'
- 理由: 着手前の確認（移行に `outage_target` が無い）なので、1.2 で移行を書いた後は必ず FAIL する（evidence.jsonl の 1.1 FAIL。head b2dfc1a1、tree dabcf21a）
- 手で直した理由: 新しい入口が `bash -c` で、plan_fix.py の「最初の語が repo の実行ファイル」を満たさない
- 承認: 2026-10-14 本人（`deep-answers-3.txt` の Q-T1「承認する」）
- 1.2 / 1.3 は直さない: `quiet-run` の包みは harness2 #73（verify-run が `quiet-run` を外して走らせる）で解消済み（本人の補足）

## 2026-10-14 10.1

- 旧: bash -c 'set -a; . ./.env; set +a; tools/stack.sh up >/dev/null && curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_PORT:-18787}/coverage?from=2026-08-01&to=2026-09-30" > /tmp/cov.json' && jq -e …（変えない）
- 新: bash -c 'set -a; . ./.env; set +a; . tools/ports.sh; rm -f /tmp/st14-stack.log /tmp/cov.json; STACK_RESET=1 setsid tools/stack.sh up >/tmp/st14-stack.log 2>&1 & pid=$!; for i in $(seq 1 300); do grep -q "^画面: http" /tmp/st14-stack.log && break; kill -0 $pid 2>/dev/null || break; sleep 2; done; curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_HTTP_PORT}/coverage?from=2026-08-01&to=2026-09-30" > /tmp/cov.json; rc=$?; kill -- -$pid 2>/dev/null; wait $pid 2>/dev/null; exit $rc' && jq -e …（同じ）
- 理由: tools/stack.sh up は前景で待つので && の後ろの curl に着かない（verify-run が 9 分以上返らなかった）。読み先の ${ASHIATO_PORT:-18787} は存在しない変数で、既定の 18787 は main の本番のサーバ
- 併せて: 本文の R13 の括弧の `tools/stack.sh` の backtick を外した（verify-run が検証コマンドとして拾い、前景で走らせる）
- 手で直した理由: 新しい入口が `bash -c` で、plan_fix.py の「最初の語が repo の実行ファイル」を満たさない
- 承認: 2026-10-14 本人（`deep-answers-4.txt` の Q-T10「1 コマンドの入口に直す」）
