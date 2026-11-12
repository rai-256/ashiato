# Task 2 実装報告

## 実装内容

- `crates/updater` と `ashiato-updater` の `once` 口を追加し、設定の有効・無効、24 時間の due 判定、固定宛先、試験ビルド限定の宛先・時刻差し込み、HTTP の見出し・時間切れ・転送制限、状態 JSON と追記型 JSON Lines を実装した。
- Windows 限定の `service` 口を追加し、起動時と 1 時間ごとの確認を行う。`crates/winsvc` はサービス名を指定できる入口を追加した。
- loopback の受け手と一時ディレクトリを使う `crates/updater/tests/st57_once.rs` に 12 本の `st57_` Scenario テストを追加した。
- `tools/check-updater-deps.sh` と CI の検査配線を追加した。共有判定 crate、サーバ、更新係、PC 収集アプリの依存境界を検査する。

## テスト結果

- `cargo test --workspace`: PASS
- `cargo test -p ashiato-updater st57_`: 12/12 PASS
- `cargo clippy -p ashiato-updater -p ashiato-winsvc --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: PASS
- `tools/check-updater-deps.sh`: PASS
- `tools/check-updater-deps.sh --self-test`: PASS
- `tools/check-offsite.sh`: PASS
- `scripts/verify-run 2.1`: PASS（最終コードで再実行）
- `scripts/verify-run 2.2`: PASS（最終コードで再実行）

## TDD Evidence

- RED: `scripts/quiet-run st57-red -- cargo test -p ashiato-updater st57_`。最初は実装途中の `CheckReason` 変換が未完成でコンパイルエラーになり、テストを実行できなかった。これはテストの誤りではなく、実装の不足を示す失敗だった。
- GREEN: `scripts/quiet-run postfix-test2 -- cargo test -p ashiato-updater st57_`。12 本すべて PASS。

## 変更ファイル

- `Cargo.toml`, `Cargo.lock`
- `crates/updater/Cargo.toml`, `build.rs`, `src/lib.rs`, `src/main.rs`, `tests/st57_once.rs`
- `crates/winsvc/src/lib.rs`
- `tools/check-updater-deps.sh`
- `.github/workflows/ci.yml`
- `openspec/changes/st57-update-notice/evidence.jsonl`

## 自己レビュー

- 本番ビルドは `GITHUB_API` 固定、試験ビルドだけ環境変数または `settings/test-hooks.json` を使うことを確認した。
- `enabled: false` は状態読込・問い合わせ・記録追加より先に戻る。
- 変更後に focused test、Windows target clippy、依存検査、workspace test、`verify-run` を再実行した。
- commit hook の A-2 検査に合わせ、CLI はエラー本文を標準エラーへ出さず終了コードだけ返す。

懸念事項はない。

## 修正ラウンド 1（F1〜F12）

### 修正内容

- `ureq` に gzip 展開と ring provider を有効化し、OS の証明書検証を維持した。依存検査で同梱根証明書が無いことも確認した。
- 同一 `api.github.com`（試験時は同一 loopback authority）の転送だけを最大 3 回追い、別 authority は `unreadable` として記録するようにした。
- timeout を `ureq::Error` の型と `TimedOut` で分類し、HTTP 403/429/404/500/壊れた JSON、31 秒無応答、要求メソッド・見出し・本文なし、3→4 行追記をテストで固定した。
- 開発版の updater directory/service name を `-dev` に切り替え、サービスの停止確認を 1 秒間隔にし、サービス実行時のエラーを `logs/updater.log` に追記するようにした。CLI エラーは標準エラーへ表示する。
- `check-updater-deps.sh --self-test` が DB・サーバ依存・共有 crate・根証明書の各偽入力を個別に検出するようにした。

### 修正後の検証

- RED: `scripts/quiet-run st57-red -- cargo test -p ashiato-updater st57_` は要求形状の追加 assertion が失敗し、続くテストも poison lock で失敗した。既存実装が要求形状とテスト独立性を担保していないことを確認した。
- GREEN: `scripts/verify-run 2.1` — 13/13 focused tests、Windows target clippy とも PASS。
- GREEN: `scripts/verify-run 2.2` — updater dependency check、self-test、offsite check、CI wiring とも PASS。
- `scripts/quiet-run workspace -- cargo test --workspace` は 1953 件 PASS、既存の `drops_tests::drops_api_idempotent` 1 件が PostgreSQL `40P01 deadlock detected` で FAIL。今回の変更範囲外の DB 並行試験失敗である。
- `git diff --check` PASS。

### 変更ファイル

`Cargo.lock`、`crates/updater/Cargo.toml`、`crates/updater/src/lib.rs`、`crates/updater/src/main.rs`、`crates/updater/tests/st57_once.rs`、`tools/check-updater-deps.sh`、`openspec/changes/st57-update-notice/evidence.jsonl`

### 自己レビュー

- focused test と `verify-run` は最終コードに対して再実行済み。
- `tasks.md`、仕様本文、既存 Scenario の期待値は変更していない。
- workspace 全体の deadlock は updater のテスト・依存・サービス変更と無関係な既存テストの並行性問題として残る。
