# Task 3 レポート

## 実装内容

- `tools/android-emulator.sh` に、起動前に `adb devices` の `emulator-*` かつ `device` の各 serial を調べる処理を追加した。
- 各エミュレータへ `timeout 5 adb -s <serial> emu avd name` を実行し、対象 AVD と一致した場合は理由を stderr に出して rc=1 で終了する。既存端末へ `emu kill` は送らず、エミュレータ起動部品も呼び出さない。
- `tools/test-android-emulator.sh` の偽 adb に実機・別 AVD の応答を追加し、Task 3 の 2 Scenario を追加した。

## TDD Evidence

### RED

- コマンド: `ASHIATO_TEST_ONLY=same-avd tools/test-android-emulator.sh`
- 結果: Scenario 印字後 rc=1。実装前は同一 AVD の検出がなく、期待する拒否条件を満たせなかった。

### GREEN

- `ASHIATO_TEST_ONLY=same-avd tools/test-android-emulator.sh && ASHIATO_TEST_ONLY=other-device tools/test-android-emulator.sh`: 2/2 成功。
- `scripts/quiet-run task3 -- tools/test-android-emulator.sh`: rc=0、28 秒。
- `scripts/verify-run 3.1`: sandbox 内は `/dev/kvm` のため BLOCKED_INFRA。規約どおり sandbox 外で再実行し、PASS（rc=0、33 秒）の証跡を `evidence.jsonl` に記録した。

## 変更ファイル

- `tools/android-emulator.sh`
- `tools/test-android-emulator.sh`
- `openspec/changes/ch10-emulator-start-timeout/evidence.jsonl`（検証証跡）

## 自己レビュー

- `bash -n tools/android-emulator.sh tools/test-android-emulator.sh` 成功。
- `git diff --check` 成功。
- 同一 AVD は `emu_start`、KVM 検査、後片付けより前に拒否される。実機は問い合わせ対象外で、別 AVD は従来どおり起動する。
- 追加の懸念なし。

## Fix round 1

- F1 を修正した。same-avd の偽端末を `emulator-5554` に変更し、stderr の同じ行に既存 AVD のエラーと `adb -s emulator-5554 emu kill` の停止案内が含まれることを検証する grep にした。
- RED: `ASHIATO_TEST_SCRIPT_REV=565531be2 ASHIATO_TEST_ONLY=same-avd tools/test-android-emulator.sh` は rc=1。旧実装では同一 AVD の拒否がなく、強化した期待を満たさないことを確認した。
- GREEN: `ASHIATO_TEST_ONLY=same-avd tools/test-android-emulator.sh` と `ASHIATO_TEST_ONLY=other-device tools/test-android-emulator.sh` がともに rc=0。
- 全体試験: `scripts/quiet-run task3 -- tools/test-android-emulator.sh` は rc=0（28 秒）。
- 項目検証: sandbox 内の `scripts/verify-run 3.1` は BLOCKED_INFRA（/dev/kvm）。sandbox 外で同じコマンドを再実行し PASS（rc=0、33 秒）、evidence.jsonl に記録した。
- 補助確認: `bash -n tools/test-android-emulator.sh`、`git diff --check` は成功。
