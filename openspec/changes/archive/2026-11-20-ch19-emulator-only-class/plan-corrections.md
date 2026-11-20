# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-19 4.3

- 旧: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/android-emulator\.sh|tools/test-android-emulator\.sh|AGENTS\.md|docs/changes/CH19\.md|openspec/changes/ch19-emulator-only-class/|docs/briefs/CH19|\.superpowers/|$)"'`
- 新: `./openspec/changes/ch19-emulator-only-class/check-allowed-paths.sh`
- 理由: review_triage が R17 の followup に docs/handoff/CH19.md を必須にするため、元の許可リストは正式な申し送りを必ず失敗として扱う
- 実証: evidence.jsonl の 2026-11-19T14:43:48+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261119-234348-evidence-4.3-0-235b2c-vgs8fteab.log、HEAD 2826a0f）
- 承認: machine: evidence.jsonl 2026-11-19T14:44:01+00:00 PASS
