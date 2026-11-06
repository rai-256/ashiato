# Task 7 実装報告

## 実装内容

- サーバの `service` 口を Argon2id の `WEB_PASSWORD_VERIFIER` 必須にし、元の `WEB_PASSWORD` との混在・欠落・不正形式を拒否するようにした。
- Argon2id verifier によるログイン、API_TOKEN / WEB_SESSION_KEY との一致検査、`ashiato-web-session-v2` の印材料を追加した。開発口の `WEB_PASSWORD` / v1 は維持した。
- インストーラに 2 回入力、5 種の理由付き検査、Argon2id PHC 化、設定への verifier 保存、`password` 口での再設定、再起動、既定ブラウザ起動を追加した。
- `argon2` と `rpassword` の依存を追加した。

## TDD Evidence

### RED

`scripts/quiet-run red -- cargo test -p ashiato-server st55_verifier_` を実行し、未実装の verifier 関数と拒否種別が無いコンパイルエラーを確認した。

`scripts/quiet-run installer-red -- cargo test -p ashiato-setup st55_password` を実行し、未作成の `password` モジュールによるコンパイルエラーを確認した。

### GREEN

- `scripts/verify-run 7.1`: PASS。verifier テスト 6 本以上、web_session テスト、ライセンス検査。
- `scripts/verify-run 7.2`: PASS。password テスト 5 本以上、`tools/release/install-check.sh --self-test`。
- focused 実行でも server verifier 6/6、installer password 5/5 が通過。

## 変更ファイル

- `crates/server/Cargo.toml`, `Cargo.lock`
- `crates/server/src/lib.rs`
- `crates/server/src/web_session.rs`
- `crates/server/src/st55_tests.rs`
- `crates/installer/Cargo.toml`
- `crates/installer/src/password.rs`
- `crates/installer/src/lib.rs`
- `crates/installer/src/main.rs`
- `crates/installer/src/secrets.rs`
- `openspec/changes/st55-server-installer/evidence.jsonl`

## 自己レビュー・懸念

- `cargo test -p ashiato-server` 全体は 1896 テスト中 1892 成功、`drops_tests::drops_api_idempotent` の DB deadlock 1 件で失敗した。変更箇所外の既存テスト失敗として記録する。
- `password` 口の管理者昇格そのものは既存の Windows 起動基盤に依存するため、Linux の `install-check.sh --self-test` と純粋な入力検査で確認した。

## Fix round 1

### 修正内容

- `install-check.sh` に Task 7.2 の 10 Scenario の印と self-test 検査を追加した。
- 合言葉の不正入力を、端末・標準入力のどちらでも再入力段へ戻すようにした。
- `password` 口に試験用の昇格拒否と Windows の `RunAs` 昇格を追加した。
- `WEB_PASSWORD_VERIFIER` だけを設定 JSON に差し替え、取り込み先など既存の鍵を保持するようにした。
- サービス停止の完了を待ってから設定更新・サービス起動するようにした。trace に秘密の語を出さないようにした。
- 決め直し後の旧印を 401 と判定するテストを明示した。

### TDD Evidence

- RED: `scripts/quiet-run red -- cargo test -p ashiato-setup st55_password_` — 未実装の `replace_web_password_verifier`、停止待ち命令、昇格命令でコンパイルエラー。
- GREEN: `scripts/quiet-run server3 -- cargo test -p ashiato-server st55_verifier_` — 6 tests passed（list 6）。
- GREEN: `scripts/quiet-run installer3 -- cargo test -p ashiato-setup st55_password_` — 8 tests passed（list 8）。

### 検証

- `tools/release/install-check.sh --self-test` — PASS。
- `tools/check-licenses.sh` — PASS（Rust 366、Node 257、不許可 0）。
- `scripts/quiet-run web-session -- cargo test -p ashiato-server web_session` — 28 tests passed。
- `scripts/quiet-run full-suite -- cargo test --workspace` — rc=0。

### 変更ファイル

- `crates/installer/src/commands.rs`
- `crates/installer/src/lib.rs`
- `crates/installer/src/main.rs`
- `crates/installer/src/secrets.rs`
- `crates/server/src/st55_tests.rs`
- `crates/server/src/web_session.rs`
- `tools/release/install-check.sh`

### 自己レビュー

- Task 7 の open findings F1〜F7 に対応した。Windows の実機サービス通しは環境上実行していないため、CI の `install-windows` で確認される。

## Fix round 2

### 修正内容

- `install-check.sh --self-test` の偽 zip に、初回ログイン、3 種の不正入力、決め直し後の新旧ログイン、旧セッション印の拒否、設定保持、昇格拒否、ブラウザ起動の実動検査を追加した。
- self-test の検査対象 state を一時ディレクトリへ引き渡し、元の合言葉を設定・記録へ残さないことと、命令記録に秘密の語を出さないことを確認するようにした。
- 前回ラウンドで既に入っていた F2〜F6（入力再試行、昇格、設定保持、停止待ち、401 検証）は変更せず、現在の木で再検証した。

### TDD Evidence

- RED: `scripts/quiet-run install-self-test-red -- tools/release/install-check.sh --self-test` — 初回ログイン検査で照合値が無く失敗（rc=1）。
- GREEN: `scripts/quiet-run install-self-test-green2 -- tools/release/install-check.sh --self-test` — rc=0、追加した 10 Scenario の検査を含め完了。

### 検証

- `scripts/quiet-run setup-tests-final -- cargo test -p ashiato-setup st55_password_` — 8/8 passed、list 8。
- `scripts/quiet-run server-verifier-tests -- cargo test -p ashiato-server st55_verifier_` — 6/6 passed、list 6。
- `scripts/quiet-run server-web-session-tests -- cargo test -p ashiato-server web_session` — 28/28 passed。
- `tools/check-licenses.sh` — PASS（Rust 366、Node 257、不許可 0）。
- `git diff --check` — PASS。

### 変更ファイル

- `tools/release/install-check.sh`

### 自己レビュー

- F1 の self-test は偽 zip 内で合言葉の決め直しと状態保持まで検査するようになった。F2〜F6 は前回修正済みで、対象テストが引き続き通過した。Windows の実機通しは引き続き `install-windows` に委ねる。

### 追加修正

- `verify-run 7.2` の初回実行で検出された self-test の一時フォルダ漏れを、state export 後も通常 cleanup するよう修正した。再実行は PASS で、漏れの注意は出ていない。

## Fix round 3

### 修正内容

- F3/F15: `password` の Windows 昇格再起動が渡す `--elevated` を引数解析で受け付け、昇格済みの子プロセスでは再昇格しないようにした。昇格拒否の試験用差し込みは従来どおり失敗として扱う。
- F7: `password` 口の trace に口名そのものを記録せず、秘密を含まない `credential-update` として記録するようにした。
- F5: 決め直し前の印が旧設定では 204 相当で通り、決め直し後の印が 401 で断られることを同じ試験で明示した。

### 検証

- `scripts/quiet-run server-verifier-lib -- cargo test -p ashiato-server --lib st55_verifier_` — 6/6 passed。`-- --list` も 6 本。
- `scripts/quiet-run installer-verifier-lib -- cargo test -p ashiato-setup --lib st55_password_` — 8/8 passed。`-- --list` も 8 本。
- `scripts/quiet-run install-check-self-test -- tools/release/install-check.sh --self-test` — rc=0。
- `scripts/quiet-run workspace-tests -- cargo test --workspace` — rc=0。
- `scripts/quiet-run licenses -- tools/check-licenses.sh` — rc=0、不許可 0 件。
- `cargo fmt --all -- --check`、`git diff --check` — PASS。

### 変更ファイル

- `crates/installer/src/commands.rs`
- `crates/installer/src/lib.rs`
- `crates/installer/src/main.rs`
- `crates/server/src/st55_tests.rs`

### 自己レビュー

- 指定された F3・F5・F7・F15 のみを修正した。Windows の実機での UAC とサービス通しはこの環境では実行しておらず、CI の `install-windows` に委ねる。
