# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認のうえで直した記録。

## 2026-10-14 10.2

- 旧: `cd web && npm run lint && npm run test && npm run build && npm run test:e2e`
- 新: `tools/db.sh down -v && cd web && npm run lint && npm run test && npm run build && STACK_RESET=1 npm run test:e2e`
- 理由: cargo test --workspace の後の同じ DB では、ST12 の既存テストが残す c03-myactivity-u<hash>（その日に始まり格子の子が 0）で web/e2e/archive-layout.spec.ts:166 が ST33 と無関係に落ちる。CI と同じく e2e の前に DB を作り直す（stack.sh の STACK_RESET=1）。
- 実証: evidence.jsonl の 2026-10-13T23:57:38+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261014-085738-evidence-10.2-3-399039.log、HEAD 5f94b84）
- 承認: 2026-10-14 本人の回答（深掘り Q10: 10.2 の検証コマンドを、e2e の前に DB を作り直す形に plan_fix で直す。deep-answers-6.txt）

## 2026-10-14 10.3

- 旧: grep -c 'Scenario: 削除の印の付いた記録は出ない' crates/server/src/export_tests.rs
- 新: `grep -c 'Scenario: 削除の印の付いた記録は出ない' crates/server/src/export_file_tests.rs`
- 理由: 印は export_file_tests.rs:854（Task 5）にあり、旧の先は 0 を返す（task-10 review 1 の F2）。印を二重に置かない規律なので印は動かさない
- 手で直した理由: 10.3 の grep は evidence.py の CMD_LIKE に当たらず検証コマンドとして抽出されない（declared が空）ので、plan_fix.py の前提（旧コマンドが declared にある）を満たさない
- 承認: 2026-10-14 本人の回答（深掘り Q11「直してよい」。deep-answers-6.txt）
