# CH25 —— CI の偶発的な赤を直す（gradle の部品の取り置き・時計のテストの許容幅）

change: `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D1（仮）. 取り置きは、既にある `actions/setup-java` の段の `cache: gradle` で行う
- D2（仮）. 配る組み立て（nightly.yml・release.yml の `package-linux`）も取り置きを使う
- D3（仮）. `chain` は、ci.yml で既に使っている `actions/cache` の段で取り置く

全 Task の ledger に `Ruling:` の行は無い。

## Task

3 Task のうち、埋めた Task は 3 本。

残した項目:
- 4.1（人間の確認待ち（確認バッチが持つ））: 4.1 確認バッチで本人に聞く（CI の上でしか見えないもの）:

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.2 `scripts/quiet-run clk -- cargo test -p ashiato-collector-windows --lib clock::tests::clock_reference` → rc=0（PASS・head 4d66754）
- 1.3 `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py clock` → rc=0（PASS・head 4d66754）
- 1.4 `bash -c 'set -euo pipefail; bin=$(cargo test -p ashiato-collector-windows --lib --no-run --message-format=json 2>/dev/null | python3 -c "import sys,json;print([j[\"executable\"] for j in map(json.loads,sys.stdin) if j.get(\"reason\")==\"compiler-artifact\" and j.get(\"executable\") and j[\"target\"][\"kind\"]==[\"lib\"]][-1])"); test "$("$bin" clock::tests::clock_reference --skip clock_reference_setup_time --list | grep -c ": test$")" -eq 5; for i in $(seq 3000); do "$bin" clock::tests::clock_reference --skip clock_reference_setup_time -q >/dev/null || { echo "落ちた: $i 回目"; exit 1; }; done; echo "3000 回 PASS"'` → rc=0（PASS・head 4d66754）
- 2.1 `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py ci` → rc=0（PASS・head 3ae3a0c）
- 2.2 `bash -c 'set -e; tools/check-vulns-wiring.sh --self-test; tools/check-vulns-wiring.sh; tools/release/windows-build-key.sh --check-workflow; tools/check-tmp-hygiene.sh --self-test; tools/check-tmp-hygiene.sh; tools/tmp-leak-check.sh --self-test; tools/release/check-guards.sh --self-test; tools/check-adr.sh --self-test; tools/check-adr.sh'` → rc=0（PASS・head 3ae3a0c）
- 2.3 `scripts/quiet-run locks -- bash -c 'tools/check-gradle-locks.sh --self-test && tools/check-gradle-locks.sh'` → rc=0（PASS・head 3ae3a0c）
- 3.1 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(\.github/workflows/(ci|nightly|release)\.yml|crates/collector-windows/src/clock\.rs|docs/changes/CH25\.md|openspec/changes/ch25-ci-flake-gradle-cache-clock-test/|docs/briefs/CH25|\.superpowers/|$)"'` → rc=0（PASS・head 2933653）

## レビュー（`review/code.md`）

- 指摘 7 件（fixed 4・rejected 3）
- 処置の re-review: 1 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
