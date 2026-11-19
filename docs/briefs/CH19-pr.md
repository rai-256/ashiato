# CH19 —— エミュレータの計測テストを 1 クラスだけ走らせる口を足す

change: `openspec/changes/ch19-emulator-only-class/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D3（仮）. ONLY の段の前に前の実行の結果を消し、段の後に渡したクラスの結果が XML にあることを確かめる
- D5（仮）. ONLY の値は完全修飾のクラス名 1 つだけ

全 Task の ledger に `Ruling:` の行は無い。

## Task

4 Task のうち、埋めた Task は 4 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `tools/test-android-emulator.sh` → rc=0（PASS・head f1fe856）
- 1.1 `bash -c 'out=$(ASHIATO_TEST_ONLY=only-base ASHIATO_TEST_BASE_REV=$(git merge-base origin/main HEAD) tools/test-android-emulator.sh) && ! grep -q "飛ばす" <<<"$out"'` → rc=0（PASS・head f1fe856）
- 1.1 `tools/test-usage-volume.sh` → rc=0（PASS・head f1fe856）
- 1.2 `tools/test-android-emulator.sh --self-test` → rc=0（PASS・head f1fe856）
- 2.1 `bash -n tools/android-emulator.sh` → rc=0（PASS・head d15391f）
- 2.1 `tools/test-usage-volume.sh` → rc=0（PASS・head d15391f）
- 2.1 `bash -c 'base=$(git merge-base origin/main HEAD) && test "$(git diff "$base" -- tools/android-emulator.sh | grep -c "^-[^-]")" -eq 0'` → rc=0（PASS・head d15391f）
- 2.2 `tools/test-android-emulator.sh` → rc=0（PASS・head b582bf3）
- 2.2 `tools/test-android-emulator.sh --self-test` → rc=0（PASS・head b582bf3）
- 2.2 `bash -c 'base=$(git merge-base origin/main HEAD); out=$(ASHIATO_TEST_ONLY=only-class ASHIATO_TEST_SCRIPT_REV=$base tools/test-android-emulator.sh 2>&1) && exit 1; grep -q "Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる" <<<"$out"'` → rc=0（PASS・head b582bf3）
- 3.1 `tools/test-android-emulator.sh` → rc=0（PASS・head cfb8128）
- 3.1 `bash -c 'ASHIATO_TEST_ONLY=only-st56 tools/test-android-emulator.sh'` → rc=0（PASS・head cfb8128）
- 3.1 `bash -c 'out=$(ASHIATO_TEST_ONLY=only-base ASHIATO_TEST_BASE_REV=$(git merge-base origin/main HEAD) tools/test-android-emulator.sh) && ! grep -q "飛ばす" <<<"$out"'` → rc=0（PASS・head cfb8128）
- 3.2 `bash -c 's=$(awk "/^### 長いコマンド/{f=1;next} /^##/{f=0} f" AGENTS.md); grep -q "harness2:long-commands" <<<"$s" && grep -q "ONLY=dev.ashiato.collector" <<<"$s" && grep -q "scripts/verify-run" <<<"$s" && grep -q "cargo test" <<<"$s" && grep -q -- "-- --list" <<<"$s"'` → rc=0（PASS・head cfb8128）
- 4.1 `bash -c 'rm -rf collector-android/app/build/outputs/androidTest-results && scripts/quiet-run emu56 -- env ONLY=dev.ashiato.collector.St56BackupTest tools/android-emulator.sh'` → rc=0（PASS・head d429a39）
- 4.1 `scripts/quiet-run emu1 -- env ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest tools/android-emulator.sh` → rc=0（PASS・head d429a39）
- 4.2 `scripts/quiet-run emu -- tools/android-emulator.sh` → rc=0（PASS・head 1ae6ebe）
- 4.3 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/android-emulator\.sh|tools/test-android-emulator\.sh|AGENTS\.md|docs/changes/CH19\.md|openspec/changes/ch19-emulator-only-class/|docs/briefs/CH19|\.superpowers/|$)"'` → rc=1（FAIL・head 2826a0f）
- 4.3 `./openspec/changes/ch19-emulator-only-class/check-allowed-paths.sh` → rc=0（PASS・head 2826a0f）

## レビュー（`review/code.md`）

- 指摘 20 件（fixed 16・followup 1・rejected 3）
- 処置の re-review: 4 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

- R17. `tools/test-usage-volume.sh:88` の「最初の connectedDebugAndroidTest」が、ONLY の塊の行（`tools/android-emulator.sh:142`）を指すようになった —— followup CH19 — `docs/handoff/CH19.md` に、全体経路に探索を限定する小さな follow-up change を記録した。CH19 の凍結済み tasks には差し戻さない。
- ch19-emulator-only-class R17 — usage-volume の「最初の Gradle 段」の探索を全体経路に限定する（`docs/handoff/CH19.md`。review/code.md で触れていない）
