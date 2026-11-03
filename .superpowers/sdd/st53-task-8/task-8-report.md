# Task 8 実装報告

## 実装

- `tools/release/check-guards.sh` に `approvals [--repo] [--reviewer]` と `agent-credential [--repo]` を追加した。
- `approvals` は `required_reviewers` の User login、`can_admins_bypass: false` を検査し、`prevent_self_review` は検査しない。
- `agent-credential` は `.permissions.admin` と `.permissions.maintain` がともに false であることを検査する。
- 偽 `gh` の詳細 Environment／リポジトリ権限応答と、指定された 7 Scenario の self-test を追加した。
- `release.yml` の publish で `rules` の後、署名前に `approvals` を実行し、workflow 検査で順序を検証するようにした。

## テスト結果

- `tools/release/check-guards.sh --self-test`: PASS (rc=0)
- `tools/release/check-guards.sh workflows`: PASS (rc=0)
- 指定 actionlint コマンド: PASS (rc=0; ローカル `actionlint` 不在のため Docker fallback)
- `bash -n tools/release/check-guards.sh`: PASS
- `git diff --check`: PASS
- `scripts/verify-run 8.1`: PASS (rc=0)
- `scripts/verify-run 8.2`: PASS (rc=0; 3 検証すべて PASS)

## TDD Evidence

### RED

新しい Scenario の self-test 呼び出しを実装前に追加し、`tools/release/check-guards.sh --self-test` を実行した。未対応の `approvals` を既存 usage が拒否し、rc=2 で失敗したため、テストが新しい振る舞いを実際に要求していることを確認した。

### GREEN

実装後に `tools/release/check-guards.sh --self-test` を実行し、全 Scenario と `OK: rules・Environment・workflow の守りを検査した` を出力して rc=0 になった。続けて `scripts/verify-run 8.1`／`8.2` も PASS した。

## 変更ファイル

- `.github/workflows/release.yml`
- `tools/release/check-guards.sh`
- `openspec/changes/st53-license-bundle/evidence.jsonl`（verify-run の証跡）

## 自己レビュー

既存の rules／environments の判定は変更していない。`prevent_self_review` は意図的に無視し、承認者の既定値は `ASHIATO_RELEASE_REVIEWER`、未設定時は repo owner とした。作業ツリーに未解決の懸念はない。
## Fix report（review 1）

- F1: `--reviewer <login>` を引数解析に追加し、承認者の選択を明示指定 → `ASHIATO_RELEASE_REVIEWER` → repo owner の順にした。self-test の承認 Scenario は明示指定を使うよう更新した。
- F2: nightly の APK 混入試験後、fake root の `ci.yml` と `nightly.yml` を正しい写しへ復元し、権限試験後には `release.yml` も復元するようにした。承認と署名の入れ替え試験の awk は、署名行を先に出力して実際に順序を反転するよう修正した。

### Fix verification

- `tools/release/check-guards.sh --self-test`: PASS (rc=0)
- `bash -n tools/release/check-guards.sh`: PASS (rc=0)
- `scripts/verify-run 8.1`: PASS (rc=0)
- `scripts/verify-run 8.2`: PASS (rc=0; self-test / workflows / actionlint の 3 検証)
- `git diff --check`: PASS

変更範囲を自己レビューし、F3/F4（Minor）は今回の未解決指摘に含まれないため変更していない。Actions の `GITHUB_TOKEN` による Environment 読み取りは、レビュー記録どおり人間の確認待ち 12.3 の範囲である。
