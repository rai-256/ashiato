# Task 11 実装報告

## 実装内容

- `docs/release.md` を追加し、鍵の作成・保管、`release` / `nightly` Environment、Secrets と `ASHIATO_CERT_SHA256`、ルールセット、最初の印、nightly 公開、壊れた版の修正、修正の枝、本人の環境の移行、正式版後の版更新、確認バッチを記載した。
- `docs/production-prep.md` に、ST52 の版の出所が `Cargo.toml` の workspace `version` であることを追記した。
- `docs/handoff/ST53.md` に最初の印・nightly 公開と ST52 完了判定の申し送りを追加した。
- `docs/handoff/ST54.md` に R11 の開発版 DB の扱いを追加した。
- `docs/handoff/ST58.md` に D5（仮）/ R14 の PC の後戻りを追加した。

## テスト結果

`scripts/verify-run 11.1` を実行し、検証3項目すべて PASS。`python3 scripts/check_scenarios.py . st52-release-channels` は `scenarios: OK`。既存の Scenario 印に関する警告は残るが、今回の変更による失敗はない。

## TDD Evidence

- RED: `docs/release.md` が存在しない状態で Task 11 の最初の grep 検証を実行し、`No such file or directory` / rc=1 を確認した。
- GREEN: `scripts/verify-run 11.1` を実行し、3項目が PASS / rc=0 になった。証跡は `openspec/changes/st52-release-channels/evidence.jsonl` に記録されている。

## 変更ファイル

- `docs/release.md`
- `docs/production-prep.md`
- `docs/handoff/ST53.md`
- `docs/handoff/ST54.md`
- `docs/handoff/ST58.md`
- `openspec/changes/st52-release-channels/evidence.jsonl`

## 自己レビュー

`git diff --check` は成功した。Task の指定文字列、申し送りの `ASHIATO_NIGHTLY_PUBLISH` / `R11` / `D5`、版番号の出所、ST52 では本物の鍵や公開を行わない境界を確認した。追加の懸念はない。

## Fix round 1

### 対応内容

- F1: 修正の枝の手順を、workspace version の更新 commit、修正 commit、枝 push、印の push、release workflow の完了待ち、枝削除の順に修正した。`version.sh plan` は印を作った後、印を push する前に実行する形にした。
- F2: `docs/handoff/ST54.md` の R11 申し送りを、移行の名前変更・削除では `Ahead` で起動・移行が止まり、中身の変更では警告だけで DB の形がずれること、nightly 印を毎日動かすと `git fetch --tags` が rc=1 になる原因は DB ではなく印の上書きであることに修正した。
- F3: `release` workflow が読む4つの Secret 名、keystore の base64 化、`ASHIATO_CERT_SHA256` の `keytool` による SHA-256 指紋取得手順を追加した。

### Fix 検証

- `scripts/verify-run 11.1` — 3項目すべて PASS / rc=0。`check_scenarios.py` は既存警告を出したが `scenarios: OK`。
- `git diff --check` — 成功。

### Fix 自己レビュー

F1〜F3 の指摘対象だけを変更し、`tasks.md`、コード、既存の合意済みテスト期待値は変更していない。追加の懸念はない。
