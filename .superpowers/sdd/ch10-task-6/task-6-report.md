# Task 6 レポート

## 実装

- `real-proof.sh` を追加し、`dies` / `timeout` / `same-avd` / `ok` の本物 SDK/KVM 検証入口を作成した。
- 一時 HOME、専用 ADB server、後片付け、`real-proof.txt` への結果記録を実装した。
- 実 emulator 37.1.11 で `emu avd name` が空になるため、`ro.boot.qemu.avd_name` fallback を `tools/android-emulator.sh` に追加した。

## テスト結果

- `bash -n ... && git diff --check`: PASS
- `scripts/quiet-run android-self-test -- tools/test-android-emulator.sh --self-test`: PASS
- `env ASHIATO_TEST_ONLY=same-avd tools/test-android-emulator.sh`: PASS
- `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout`: PASS（scenarios: OK）
- `real-dies`: PASS
- `real-timeout`: PASS
- `real-same`: PASS（実 emulator、KVM あり）
- `real-ok`: PASS（実 emulator、KVM あり）

## TDD Evidence

- RED: `test -x openspec/changes/ch10-emulator-start-timeout/real-proof.sh` が rc=1。
- GREEN: `bash -n`、fake self-test、`real-dies`、`real-timeout` が PASS。

## 変更ファイル

- `openspec/changes/ch10-emulator-start-timeout/real-proof.sh`
- `openspec/changes/ch10-emulator-start-timeout/real-proof.txt`
- `tools/android-emulator.sh`

## Task gate 再検証

- `HX_CHANGE=ch10-emulator-start-timeout scripts/verify-run 6.1`: 4件すべて PASS。same-avd は 23 秒、ok は 1143 秒で完了した。
- `HX_CHANGE=ch10-emulator-start-timeout scripts/verify-run 6.2`: 3件すべて PASS（`scenarios: OK`）。
- 証跡は `openspec/changes/ch10-emulator-start-timeout/evidence.jsonl` に記録した。

## 今回の修正

- 実 emulator が専用 `ANDROID_ADB_SERVER_PORT` を引き継がないため、same-avd/ok では既定 ADB server を使うようにした。
- AVD 名の検出は `ro.boot.qemu.avd_name` を短いタイムアウトで先に確認し、`emu avd name` を fallback にした。
- テスト終了時に emulator が一時 AVD を解放するまで後片付けを再試行し、ok の終了時競合を解消した。

## 懸念

検証時にハーネスが管理する一時フォルダ内へ emulator 由来のログ・設定ファイル残存を警告したが、各検証は PASS し、Task gate の証跡に記録されている。

## Fix round 1（review 1）

- F1/F2: timeout の `set -e` と各 assertion の明示的な return、台本 PID（timeout wrapper の子）の生存確認、process group と baseline を使った自分の qemu だけの残存確認に修正した。
- F3/F5: `ro.boot.qemu.avd_name` を本線として D3 を更新し、`emu avd name` の生の 1 行目も別キーで記録するようにした。偽 adb と same-avd self-test が getprop 本線を通る。
- F4: `/dev` 全体を置換せず、`/dev/kvm` だけへ `/dev/null` を bind mount する形にし、D17 の KVM 失敗行と使用形を記録するようにした。
- F6/F7: same-avd は一時 AVD、動的 port、専用 serial を使い、EXIT trap で自分の emulator/qemu と一時木だけを後片付けするようにした。`emulator-start.sh` は自分の子孫だけを再帰的に停止する。

### Fix round verification

- RED: review で指摘された旧実装は、timeout の assertion を `set +e` のまま通し、same-avd の EXIT trap は関数ローカル変数で `unbound variable` になった。修正前の正規 `verify-run 6.1` は timeout/same-avd が FAIL。
- GREEN: `bash -n ... && git diff --check` PASS、`tools/test-android-emulator.sh --self-test` PASS、same-avd/timeout の実証ケース PASS、`python3 scripts/check_scenarios.py . ch10-emulator-start-timeout` は `scenarios: OK`。
- `verify-run 6.1` の再実行では dies/ok PASS、修正後の timeout/same-avd は個別に PASS。証跡は `evidence.jsonl` に追記された。

## Fix round 2（review 1 の未解決指摘）

### 修正内容

- timeout 検証で、台本の子プロセスから実 emulator の `EMU_PID` を特定し、その PID の終了と `-avd ashiato-api35` の残留を、実行前の PID を除外して確認するようにした。
- same-avd 検証は実 `~/.android/avd` と既定 adb server を使い、動的 port/serial で自分の emulator を識別するようにした。他の worktree の emulator を `emu kill` しないよう、後片付けは自分の serial と PID の子孫だけを対象にした。
- `real-proof.txt` の `same-avd-name` を `emu avd name` の生の1行、`same-avd-getprop-name` を getprop の値として記録する形に改めた。D3 の既存設計（getprop 本線、emu avd name fallback）と fake adb の getprop シナリオは維持した。

### Fix round 2 verification

- `bash -n openspec/changes/ch10-emulator-start-timeout/real-proof.sh && git diff --check`: PASS
- `tools/test-android-emulator.sh --self-test`: PASS
- `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout`: PASS（`scenarios: OK`）
- `HX_CHANGE=ch10-emulator-start-timeout scripts/verify-run 6.1`: PASS（dies / timeout / same-avd / ok の4件、現行コード）
- `HX_CHANGE=ch10-emulator-start-timeout scripts/verify-run 6.2`: PASS（3件、現行コード）
- `real-proof.txt`: `dies`, `timeout`, `same-avd`, `ok` がすべて `pass`。`same-avd-name` と `same-avd-getprop-name` はいずれも `ashiato-api35`、加速不足文言の件数は `same-avd`/`ok` とも 0。

### 懸念

- `verify-run 6.1` は各ケースの後に `adb.0.log`、`android-yosis`、`hostapd_*.conf` などの残存を警告した。実 emulator/adb が生成する外部状態で、各検証は PASS し、今回の検証スクリプトが作った一時 AVD 木と自分の emulator は後片付け済みである。
