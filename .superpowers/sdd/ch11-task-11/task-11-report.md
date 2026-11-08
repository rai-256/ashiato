# Task 11 レポート

## 実装内容

- `AGENTS.md` の「実装と計画の規約」に、依存変更時の `--write-locks`、互換範囲での更新、例外の理由と 90 日以内の期限、PR 本文への一覧、文書へ依存名を書かない規則を追加した。
- `docs/testing.md` の既存 CI 検査表に、指定された依存脆弱性・配線・PostgreSQL minor・Gradle lock・ライセンス検査の導線を反映した。
- Scenario の印を確認した。未完了の Task 6.1 が担う `設定を API で読み返す` だけは、実装のない印を追加せず不足として残した。

## テストと検証

- `scripts/verify-run 11.1`: PASS、rc=0。
- `scripts/verify-run 11.2`: FAIL、rc=1。`設定を API で読み返す` の印が無い。実際の担保元である `tools/check-vulns.sh github-settings` は Task 6.1 が未完了。
- `openspec validate ch11-dependency-vulnerability-scan --strict`: PASS。
- `git diff --check`: PASS。

## TDD Evidence

この Task は既存の検査スクリプトや本番コードを変更せず、運用文書と既存 Scenario の網羅検査を対象とするため、新規の振る舞いテストを追加する TDD 対象ではない。指定された網羅検査は実行し、上記の不足を再現した。

## 変更ファイル

- `AGENTS.md`
- `docs/testing.md`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`
- `.superpowers/sdd/ch11-task-11/task-11-report.md`

## 自己レビュー

- Task 11.1 の指定文字列は両文書に存在し、既存の表・節の構造を壊していない。
- Task 11.2 は未完了の Task 6.1 に依存している。空の印で検査を通す変更は実際の API 検査を担保しないため行っていない。
