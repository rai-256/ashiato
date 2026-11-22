# Task 7: Windows の CI の通し（D5）

## 実装

- `tools/release/install-check.sh` の `scenario_names` に ST59 の Windows 通し Scenario 3 本を追加した。
- インストール後に画面から `POST /web-password` と `POST /collector-token` を実行し、`server.json` の対象鍵だけが変わり、他の設定と settings の ACL が維持されることを検査するようにした。
- サービスを起動し直し、新しい画面の合言葉でログインでき、旧合言葉ではログインできないこと、旧収集アプリ値の記録を拒み新しい値の記録を受け入れることを検査するようにした。
- 画面で変更した後に `ashiato-setup.exe password` で合言葉を戻せること、戻しても画面変更後の `API_TOKEN` が維持されることを検査するようにした。
- `crates/installer/src/commands.rs` の一時書き込みコメントを、`server.json` はサーバも書く実態に合わせて修正した。

## TDD Evidence

### RED

- 実行: `tools/release/install-check.sh --self-test`
- 結果: `NG: Scenario の印が実地の確かめに 1 つでない（0）: 変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない`
- Scenario 名を `scenario_names` に追加した時点で実地ブロックの印が無く、self-test が期待どおり失敗した。

### GREEN

- 実行: `tools/release/install-check.sh --self-test`
- 結果: `OK: installer の確かめの部品と Scenario の印の置き場を確認した`
- 実行: `scripts/verify-run 7.1`
- 結果: self-test と `--list` の 3 本照合がともに `PASS rc=0`。証跡は `openspec/changes/st59-change-password-from-web/evidence.jsonl`。

## テスト

- `scripts/verify-run 7.1`: PASS（2/2 検証、rc=0）。
- `scripts/quiet-run installer-test -- cargo test -p ashiato-setup`: rc=0（この package に test target がなく 0 tests）。
- `bash -n tools/release/install-check.sh`: PASS。
- `git diff --check`: PASS。

## 変更ファイル

- `tools/release/install-check.sh`
- `crates/installer/src/commands.rs`
- `openspec/changes/st59-change-password-from-web/evidence.jsonl`

## 自己レビュー

- Scenario 名は `--list` に各 1 本あり、実地本体の印も各 1 本あることを確認した。
- self-test の秘密観測検査に新しい試験用合言葉を追加し、失敗時の観測に値が出ない経路を維持した。
- Windows 実機の install-windows はこの環境では実行していない。Task の指定どおり PR CI が担当する。

## Fix report（review 1 / F1）

- 修正: `tools/release/install-check.sh` の「画面で変えた後も、インストーラで決め直せる」で、`ashiato-setup.exe password` に渡す 2 行を `PW3` と `PW2` から `PW2` と `PW2` に変更した。`password` の非対話入力は新しい合言葉と確認入力だけを読むため、これで一致した合言葉を渡す。
- 検証: `scripts/verify-run 7.1`
- 結果: `tools/release/install-check.sh --self-test` が PASS（rc=0）、Scenario 3 本の `--list` 照合が PASS（rc=0）。証跡は `openspec/changes/st59-change-password-from-web/evidence.jsonl` に追記された。
- 実地の Windows `install-windows` は Task の指定どおり PR CI で確認する。
