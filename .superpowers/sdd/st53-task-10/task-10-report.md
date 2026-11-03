# Task 10 レポート

## 実装内容

- `.github/workflows/ci.yml` の `web` job を `check-licenses.sh rust node` に限定し、`package-linux` に Android 許諾検査と notices の自己試験を追加した。
- `chain` job に `verify-prep-device.sh --self-test`、`check-spdx.sh --self-test`、`check-spdx.sh` を追加した。
- `docs/release.md` を ST53 の手順に更新し、エージェントの資格情報、承認者、鍵・Secrets の順序、印の確認、PR 成果物の掃除、head commit での下書き、携帯を接続しない確認バッチを記載した。
- `docs/testing.md` と `CONTRIBUTING.md` の Web 検査を Rust/Node 節指定に更新し、`docs/production-prep.md` と `CLAUDE.md` に ST53 の確認・`licenses/` の置き場を追記した。

## テスト結果

- `scripts/verify-run 10.1`: PASS（actionlint fallback、workflow の配置、`check-guards.sh workflows`）。
- `scripts/verify-run 10.2`: PASS（3 検査）。
- `scripts/verify-run 10.3`: PASS（文書検査、`check_scenarios.py`）。
- `git diff --check`: PASS。

## TDD Evidence

この Task は CI・文書・設定の変更で、専用の本体テストを追加する種類ではない。指定検証をテストとして先に実行した。

### RED

編集前に workflow の配置検査を実行し、`check-licenses.sh rust node` が無いとして失敗した。また `docs/release.md` の資格情報語句検査も `missing: エージェントの資格情報` で失敗した。

### GREEN

実装後に `scripts/verify-run 10.1`、`10.2`、`10.3` を実行し、各項目の全検証が PASS になった。再検証中に準備環境の一時的な失敗が一度あったが、10.1 を再実行して PASS を記録した。

## 変更ファイル

- `.github/workflows/ci.yml`
- `docs/release.md`
- `docs/testing.md`
- `docs/production-prep.md`
- `CONTRIBUTING.md`
- `CLAUDE.md`
- `openspec/changes/st53-license-bundle/evidence.jsonl`
- `.superpowers/sdd/st53-task-10/task-10-report.md`

## 自己レビュー

指定された検査の配置、文書中の順序、手元検査のコマンドを確認した。`docs/production-prep.md` の追記は既存の表形式を維持している。残る actionlint の標準コマンド未導入は、検証コマンド指定どおり Docker fallback で PASS している。

## Fix round 1

レビューの未解決指摘 F1〜F4 に対応した。

- F1: `package-linux` に `tools/check-licenses.sh --self-test` を追加した。
- F2: `docs/release.md` に ST52 の鍵検証・控え、IMP-01/IMP-02 の判断、ST55 訂正、最初の印・nightly・修正枝の具体手順、完全な keytool コマンドを復元した。
- F3: 確認バッチを 11.1〜11.5 として、draft.sh の版番号、印の前後比較、配布物 3 点と APK の `assets/licenses/`、`Commit:` / `head_sha`、`st53-probe`、LICENSE 表示、head の取得確認まで具体化した。
- F4: `docs/production-prep.md` の ★ を「依存ライセンスの検査」表セル内へ移し、表を連続させた。

再検証:

- `scripts/verify-run 10.1`: PASS（3/3、`evidence.jsonl` に 10.1 の記録）
- `scripts/verify-run 10.2`: PASS（3/3、`evidence.jsonl` に 10.2 の記録）
- `scripts/verify-run 10.3`: PASS（2/2、`evidence.jsonl` に 10.3 の記録）
- `git diff --check`: PASS
