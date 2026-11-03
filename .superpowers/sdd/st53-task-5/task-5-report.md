# Task 5 実装報告

## 実装

- `tools/release/notices.sh` を追加した。
  - Rust の `cargo tree` 依存、`rust-std`、server の production Node 依存から、名前順の notices を生成する。
  - 部品の LICENSE/COPYING/NOTICE、SPDX fallback、authors fallback を扱い、資料不足時は出力を作らず失敗する。
  - `apk` では AAR のライセンスと、名指しの GMS 4 部品の `third_party_licenses.txt` を収録する。
  - `source` と `commit` の ref/commit 検証、atomic output、自己試験を実装した。
- `tools/licenses/texts/Apache-2.0.txt` を、現行 Rust 依存で直接の許諾ファイルが無い `uiautomation` 等の fallback として追加した。
- `evidence.jsonl` に 5.1〜5.3 の PASS 証跡を追加した。

## TDD Evidence

- RED: `tools/release/notices.sh source ...` を期待する一時テストを実行し、未実装のため `No such file or directory` / rc=127 になった。
- GREEN: `tools/release/notices.sh --self-test` が全 Scenario を通過した。

## 検証

- `scripts/verify-run 5.1` — PASS
- `scripts/verify-run 5.2` — PASS
- `scripts/verify-run 5.3` — PASS（2 コマンドとも PASS）
- `bash -n tools/release/notices.sh` — PASS

## Self-review

差分を確認し、出力生成は一時ファイル経由、失敗時の一時ファイル掃除、偽 Rust 入力、Node の `dev: true` 除外、40 桁 commit 検証を確認した。

`scripts/check_scenarios.py .` は、Task 5 以外の後続 Task が担保する Scenario も未実装のため Story 全体として FAIL するが、Task 5 の自己試験 Scenario は登録されている。

## Fix round 1

- F1/F2/F6: 自己試験が一時的な main 木と PR head 木を作り、`commit` 副コマンドと `git rev-parse HEAD` の一致、および `SOURCE.txt` の commit を確認するよう修正した。不正値は `ASHIATO_BUILD_COMMIT=main` と空の commit 引数で確認する。
- F3: APK の入力を Task 4.1 の POM 一覧に限定し、AAR/JAR の許諾、SPDX fallback、GMS 4 部品の JSON の `start`/`length` 切り出しを実装した。キャッシュ全体の走査 fallback は削除した。
- F4/F5/F7/F8: 全項目をまとめて名前順に並べ、Node の author fallback、失敗部品名の確認、`MIT`/`ISC` を含む SPDX 識別子の抽出を修正した。Android SDK と BSD-3-Clause の fallback 全文も追加した。

### Fix verification

- `scripts/verify-run 5.1` — PASS（現行 tree）
- `scripts/verify-run 5.2` — PASS（現行 tree）
- `scripts/verify-run 5.3` — PASS（2 コマンドとも現行 tree）
- `tools/release/notices.sh --self-test` — PASS
- `git diff --check` / `bash -n tools/release/notices.sh` — PASS

検証コマンドが `$TMPDIR` に作る出力ファイルは verify-run の仕様上記録されるが、台本自身が生成する APK 用の POM 一覧は終了時に削除する。
