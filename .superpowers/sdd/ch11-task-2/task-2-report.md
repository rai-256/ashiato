# Task 2: 例外の表の行の検査

## 実装

- `tools/check-vulns.py` に、例外行の理由・期限・UTC基準の期限切れ・90日上限・理由の接頭辞を検査する処理を追加した。
- スキャン結果に存在しない例外 ID（別名を含む）は `勧告がもう出ない` として rc=1 にした。
- `tools/check-vulns.sh --self-test` に、Task 2 の7 Scenarioを追加した。

## テスト

- `tools/check-vulns.sh --self-test`: rc=0
- `scripts/verify-run 2.1`: PASS、rc=0（現行実装コミット `b81ed5b90` に対する証跡）
- `bash -n tools/check-vulns.sh`: PASS

## TDD Evidence

### RED

- コマンド: `tools/check-vulns.sh --self-test`
- 実装前に追加した最初の Scenario（理由の無い行）で停止した。
- 既存実装は必須欄の不備を rc=2 の一般エラーとして扱い、Scenario が要求する rc=1 と `理由が無い` の出力を満たさなかったため、期待どおりの失敗だった。

### GREEN

- コマンド: `scripts/verify-run 2.1`
- 結果: `PASS rc=0 tools/check-vulns.sh --self-test`
- Task 2 の7 Scenarioを含む自己試験全体が完走した。

## 変更ファイル

- `tools/check-vulns.py`
- `tools/check-vulns.sh`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`

## 自己レビュー

- 期限判定はローカル時刻ではなく UTC の日付を使用し、90日ちょうどを許可している。
- 未使用例外の判定は勧告 ID と aliases の両方を対象にしている。
- Task の範囲外の `tasks.md` や仕様本文は変更していない。

## 懸念

なし。
