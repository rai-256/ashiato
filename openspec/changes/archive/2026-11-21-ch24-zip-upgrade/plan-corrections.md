# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-20 1.5

- 旧: `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'`
- 新: `./openspec/changes/ch24-zip-upgrade/check-fixtures.sh`
- 理由: 旧コマンドは囲みを「最初に入れた commit」と比べるため、zip を上げる前の Task 1 の中での正当な直し（review 1 の F2〜F4 が囲みの中の材料と期待値の直しを求めた。Cargo.lock の zip は HEAD でも 2.6.1）も「作り直し」として落とし、Task 1 を完了できない。新しい入口は、囲みを入れた commit の zip が 2.6.1 であることに加え、Cargo.lock の zip が 2.6.1 だった最後の版の囲みと今の囲みを比べる（上げた後の書き直しと未 commit の変更は落ちる。一時の worktree で zip を 9.0.5 にした commit の後に囲みを変えると rc=1 を確かめた）
- 実証: evidence.jsonl の 2026-11-20T14:56:34+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261120-235634-evidence-1.5-0-953b91-v9yd50dq2.log、HEAD 1ec847a）
- 承認: machine: evidence.jsonl 2026-11-20T14:57:42+00:00 PASS

## 2026-11-21 4.2

- 旧: `bash -c 'set -o pipefail; for f in crates/server/src/export_file_tests.rs crates/server/src/archive_tests.rs crates/collector-windows/src/update_apply.rs; do c=$(git log --format=%H -S CH24-FIXTURE-BEGIN -- "$f" | tail -1); test -n "$c" || { echo "囲みが無い: $f"; exit 1; }; git show "$c:Cargo.lock" | python3 -c "import sys,re;t=sys.stdin.read();v=re.findall(r\"name = \\\"zip\\\"\\nversion = \\\"([^\\\"]+)\\\"\",t);print(v);assert v==[\"2.6.1\"],v" || exit 1; diff <(git show "$c:$f" | sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p") <(sed -n "/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p" "$f") || { echo "囲みの中が変わった: $f"; exit 1; }; done'`
- 新: `./openspec/changes/ch24-zip-upgrade/check-fixtures.sh`
- 理由: 旧コマンドは Task 1 内で正当に更新された fixture を最初の commit と比較して誤検出する。zip 2.6.1 の最後の版と比較する既存の入口が同じ受け入れ条件を成立させる。
- 実証: evidence.jsonl の 2026-11-20T16:13:53+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261121-011353-evidence-4.2-0-953b91-vc3c67ycn.log、HEAD 3964d1d）
- 承認: machine: evidence.jsonl 2026-11-20T16:14:45+00:00 PASS
