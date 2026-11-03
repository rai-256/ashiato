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

## Fix round 1 対応

- F1: `runtime.rs` に、開発版の `state_dir` 直下に除外ファイルが無いことを確認し、親置き場の `exclusions.json` を履歴取得へ渡して `example.test/1` を除外し `/2` を送る Scenario テストを追加した。
- F2: main の除外読み込みを `load_engine` に切り出し、その関数から Engine へ除外が届き題名「パスワード管理」が記録されないことを検証する Scenario テストを追加した。
- F3: 正式版と開発版の `last-seen.txt` をそれぞれ実際に作成し、未送信ファイルのパスも別になることを検証した。
- F4/F5: 版番号から求めた正式版の `entry_name`・AUMID・表示名を検証し、AUMID/表示名は Windows API 非依存の `toast_names` へ移して開発版の値を検証した。
- F6: 正典に無い `自動起動の設定は、渡された値をそのまま書く` の印を削除した。
- F7: `Runtime::new`、autostart install、toast register が呼び出し側から `Channel` を受け取り、main が版番号から求めた同じ値を渡すようにした。既存の Runtime 呼び出しと browser history example も更新した。

### Fix round 1 の検証

- RED: `scripts/quiet-run test-red -- cargo test -p ashiato-collector-windows st54_main_loads_foreground_exclusions_into_engine` は、実装途中の `Runtime::new` の引数不足（7 引数に対し 6 引数）で rc=101。これは系統引数を導入した未完状態による期待どおりの失敗。
- GREEN: `scripts/verify-run 6.1` は 2 検証とも PASS、st54 テスト列挙は 12 本、全体テスト rc=0。
- GREEN: `scripts/verify-run 6.2` は `tools/release/build.sh --self-test` rc=0。
- 補足: `cargo check -p ashiato-collector-windows --all-targets` rc=0、`git diff --check` 問題なし。
