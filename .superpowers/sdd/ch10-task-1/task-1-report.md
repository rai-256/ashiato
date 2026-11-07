# Task 1 report

## 実装

- `tools/test-android-emulator.sh` を追加した。
- 本物の `tools/android-emulator.sh`、`tools/usage-volume.sh`、`tools/test-usage-volume.sh` を使い捨て木へ写し、偽の SDK、adb、avdmanager、emulator、media fixture、gradle を隣に置く。
- 正常起動の引数、6 段の XML、アニメーション設定、通常の後片付け、`KEEP=1`、emulator/システムイメージ欠落、AVD 作成、`tools/test-usage-volume.sh` の既存検査を Scenario 印つきで固定した。
- `--self-test` では写しだけから `-no-snapshot` と KEEP の扱いを壊し、試験がそれぞれ検出できることを確認する。
- 一時木と偽 emulator の子プロセスは `trap` で後片付けする。

## テスト結果

- `tools/test-android-emulator.sh`: PASS（rc=0、6 Scenario 実行）
- `tools/test-android-emulator.sh --self-test`: PASS（rc=0）
- `bash -n tools/test-android-emulator.sh`: PASS
- `scripts/quiet-run task1 -- bash -c 'tools/test-android-emulator.sh && tools/test-android-emulator.sh --self-test && bash -n tools/test-android-emulator.sh && git diff --check'`: PASS（rc=0）
- `scripts/verify-run 1.1`: BLOCKED_INFRA（Codex sandbox で `/dev/kvm` を開けず、未実行）
- `scripts/verify-run 1.2`: BLOCKED_INFRA（Codex sandbox で `/dev/kvm` を開けず、未実行）

## TDD Evidence

### RED

`tools/test-android-emulator.sh` 作成前に `tools/test-android-emulator.sh` を実行し、`rc=127` と `No such file or directory` を確認した。対象試験が存在しないための期待どおりの失敗である。

### GREEN

実装後、`tools/test-android-emulator.sh` は 6 個の Scenario 印を出して rc=0、`tools/test-android-emulator.sh --self-test` は rc=0 になった。自己試験ではコピーした台本だけを壊し、正常経路の引数と KEEP の検査が失敗することを確認している。

## 変更ファイル

- `tools/test-android-emulator.sh`
- `openspec/changes/ch10-emulator-start-timeout/evidence.jsonl`
- `.superpowers/sdd/ch10-task-1/task-1-report.md`

## Self-review

- 既存の Android 台本、採寸台本、共有 Scenario 台本は変更していない。
- 偽の adb は引数を記録し、偽 emulator は `FAKE_EMU` の状態を持つ。Task 1 の正常経路固定に必要な範囲に限定している。
- 正式 `verify-run` は環境制約により PASS を取得できていないため、Task gate 側で BLOCKED_INFRA として扱う必要がある。

## Issues / concerns

`verify-run` の 2 項目は `/dev/kvm` 不可で走らず、sandbox 外再実行の申請も環境側で aborted になった。これは試験の失敗ではなく、証跡に記録されたインフラ制約である。

## Fix report: review 1 F1〜F4

### 変更

- 偽 emulator の子と前景 sleep を使い捨て木の絶対パスおよび `ch10-fake-emulator` を argv に持つ形で起こし、既存の `pgrep -f "$root/"` cleanup が回収できるようにした。
- `run_emulator` に場合の鍵を渡し、timeout の rc=124 を `見張りで打ち切った: <場合の鍵>` として報告するようにした。
- missing emulator、missing system image、AVD 作成の判定を `|| return 1` で失敗伝播する形にした。
- self-test が壊した写しを通常の `assert_normal` / `assert_keep` に渡し、同じ Scenario 判定で `-no-snapshot` と KEEP の破壊を検出するようにした。各 assert の判定も非 0 を返すようにした。

### TDD Evidence

#### RED

F4 の判定を通常 assert に統一した直後、まだ assert が最後の grep の終了値を返していた段階で、`tools/test-android-emulator.sh --self-test` を実行した。`self-test: -no-snapshot の破壊を検出できない`、rc=1 となり、壊した写しを通常判定へ渡しても失敗を検出できない欠陥を再現した。

#### GREEN

- `tools/test-android-emulator.sh --self-test`: rc=0
- `tools/test-android-emulator.sh`: rc=0、6 Scenario 印
- `scripts/quiet-run task1-fix -- bash -c 'tools/test-android-emulator.sh && tools/test-android-emulator.sh --self-test && bash -n tools/test-android-emulator.sh && git diff --check'`: rc=0
- `scripts/verify-run 1.1`: PASS、rc=0（sandbox 外、ログ `20261107-124444-evidence-1.1-0-f5649a-vued71q7r.log`）
- `scripts/verify-run 1.2`: PASS、rc=0（sandbox 外、ログ `20261107-124458-evidence-1.2-0-aa9140-v2k6wl9j2.log`）

### Fix 後の self-review

対象の F1〜F4 だけを修正した。`tools/test-android-emulator.sh` と正式検証の `evidence.jsonl` が変更対象であり、元の Android 台本や tasks.md は変更していない。self-test は通常の assert 関数を直接使うため、後から通常判定を緩めた場合にも同じ検出経路で失敗する。
