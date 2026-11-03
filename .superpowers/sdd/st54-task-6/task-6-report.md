# Task 6 実装報告

## 実装内容

- `Channel` を版番号から起動時に決め、PC 収集アプリの `Config`、`Runtime`、main の組み立てに反映した。
- 開発版の状態置き場を `<ASHIATO_STATE_DIR>/dev`、端末識別子を `-dev` 付きにし、除外ファイルは親置き場から読むようにした。
- 前景とブラウザ履歴の両方が `Config::exclusions_path` を使うようにした。
- 開発版の自動起動項目、通知 AUMID/表示名、配布 zip 内の実行ファイル名を正式版と分離した。

## テスト

- RED: `cargo test -p ashiato-collector-windows st54_ -- --nocapture`。新しい `Config` フィールドと系統引数が未実装だったためコンパイルエラーになった。
- GREEN: `scripts/verify-run 6.1`。st54 テスト件数条件およびパッケージ全体が PASS（rc=0）。
- GREEN: `scripts/verify-run 6.2`。`tools/release/build.sh --self-test` が PASS（rc=0）。
- 追加確認: `scripts/quiet-run st54-final -- cargo test -p ashiato-collector-windows` が PASS（rc=0）。

## 変更ファイル

`crates/collector-windows/` の設定・自動起動・通知・Runtime・履歴呼び出し箇所、`tools/release/build.sh`、検証証跡を変更した。

## 自己レビュー

正式版の既定値と自動起動へ書く元の環境値を保持し、開発版だけを派生させることを確認した。`zipinfo` を使って配布物の実行ファイル名を検証している。

## 懸念

Windows 実機専用テストは Linux では対象外のため、このセッションでは実行していない。Task 6 の指定検証はすべて PASS している。
