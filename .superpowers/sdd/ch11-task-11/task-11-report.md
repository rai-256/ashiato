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

## 継続対応（11.2 の FAIL 解消）

- FAIL の根因だった Task 6.1 の未実装を埋めるため、`tools/check-vulns.sh github-settings` を追加した。GitHub API の4項目（alerts 204、SBOM 200、自動更新無効、`dependabot.yml` 不在）を確認し、権限不足は rc=2 とする。
- `tools/check-vulns.sh --self-test` に偽の `gh` を使う `設定を API で読み返す` Scenario を追加した。
- RED: `scripts/quiet-run task11-github-settings-red -- tools/check-vulns.sh --self-test` は rc=1（入口未実装）で失敗。
- GREEN: `scripts/quiet-run task11-github-settings-green2 -- tools/check-vulns.sh --self-test` は rc=0（41 行）で成功。
- `scripts/verify-run 11.2`: PASS、rc=0、`scenarios: OK`（fp=`5ddd6d0cc5` の不足は解消）。

## TDD Evidence

この Task は既存の検査スクリプトや本番コードを変更せず、運用文書と既存 Scenario の網羅検査を対象とするため、新規の振る舞いテストを追加する TDD 対象ではない。指定された網羅検査は実行し、上記の不足を再現した。

## 変更ファイル

- `AGENTS.md`
- `docs/testing.md`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`
- `.superpowers/sdd/ch11-task-11/task-11-report.md`
- `tools/check-vulns.sh`

## 自己レビュー

- Task 11.1 の指定文字列は両文書に存在し、既存の表・節の構造を壊していない。
- Task 11.2 は未完了の Task 6.1 に依存している。空の印で検査を通す変更は実際の API 検査を担保しないため行っていない。

## Fix round 1

### 対応

- F1: `docs/testing.md` の CI 表から `rust` job の脆弱性検査の誤記を外し、`vulns` job（PR かつ `deps`、self-test / `--base` / dependency-review）の行を追加した。
- F2: Scenario の印を偽 `gh` の self-test から `github-settings` 本体へ移し、SBOM が取れない場合の `security_and_analysis` PATCH と再取得を実装した。
- F3: GitHub API の 401/403 のみ rc=2（管理者権限不足）とし、404・その他の API/通信失敗は状態と rc を出して rc=1 にした。

### 検証

- 偽 `gh` による成功、404（rc=1）、403（rc=2）、SBOM 404 後の PATCH 再取得（rc=0）を確認した。
- `scripts/quiet-run task11-vulns-self -- tools/check-vulns.sh --self-test`: rc=0、40 行。
- `scripts/verify-run 11.1`: PASS、rc=0。
- `scripts/verify-run 11.2`: PASS、rc=0、`scenarios: OK`。
- `openspec validate ch11-dependency-vulnerability-scan --strict`: PASS。
- `bash -n tools/check-vulns.sh`、`git diff --check`: PASS。

### TDD Evidence

- RED: 修正前ファイル（`git show HEAD^:tools/check-vulns.sh`）に偽 `gh` の HTTP 404/rc=1 を与えると、`red_rc=2` かつ「管理者の権限が要る」になった。これは 404 を権限不足と誤分類する失敗。
- GREEN: 修正後の偽 `gh` 検証で 404 は rc=1 と HTTP 404 を出し、403 は rc=2、SBOM 404 後の PATCH 経由は rc=0 になった。

### Fix round self-review

- Scenario 名は 50 本の内訳を増やさず、本体の `github-settings` に 1 箇所だけ残した。
- Task 6.1 の本物の GitHub 設定有効化は管理者資格が必要なため実行していない。偽 `gh` の自己試験とローカルの状態分岐のみを検証した。
