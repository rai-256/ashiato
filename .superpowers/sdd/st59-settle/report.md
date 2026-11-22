# settle 2026-11-22

- R1/R8/R12: permit 取得後に秘密組を取得し、permit を blocking 検証へ所有移動した。`cargo test -p ashiato-server st59_both_`: 8 passed。
- R7: install-check が合言葉変更応答の Cookie jar を保存するよう変更した。
- R9/R10/R13: 成功監査行を設定保存後にし、CPU/ファイル操作を blocking 側へ移し、throttle の早期判定を追加した。`cargo test -p ashiato-server st59_both_`: 8 passed。
- R11: 一時ファイルへ元の Unix mode を引き継ぎ、失敗時に削除するよう変更した。`cargo check -p ashiato-server`: rc=0。
# settle 再処置（2026-11-22）

- R2 / R14: `LOGIN_CHECK` を照合後も 2 つの変更口が `replace_secrets` まで保持するようにし、照合用の blocking task は guard を返す形にした。旧い組で通ったログインや変更どうしが差し替えの間に割り込まない。
- R3: 15 文字の `too_short`、16 文字の受理、全 5 理由文字列を `st59_web_password_handler_applies_all_five_validation_rules` で固定した。
- R4: 再起動試験は `service::load_settings` で保存済み verifier を読み、`with_web_login_verifier` で組み立て直す形に変えた。
- R5: `st59_both_change_values_are_not_logged` は `tracing` subscriber の出力も捕捉し、成功・失敗時の合言葉、旧 API token、新 token が無いことを確認する。
- R6: 収集アプリ token と実在する衛星資格情報に正しい現在の合言葉を載せても `/web-password` が 403 となり、`wrong_password` ではないことを確認する。
- R7: Windows install check の Netscape cookie jar を `#HttpOnly_` を domain として保持し、実タブで 7 列を分解して `ashiato_session` の値を取得するよう修正した。
- R15 / R16: `replace_setting_with_previous` は読み出しと原子的書換えを blocking 側で行い、session INSERT または access-log 書込み失敗時は `restore_setting` の原子的置換で値を戻す。
- R17: access-log 失敗試験の JSON 比較は `replace_setting_at` が pretty JSON へ正規化するためバイト列が変わることを除外し、設定の意味（全キーと値）が不変であることを観測するためである。

検証:

- `cargo check -p ashiato-server --tests` rc=0
- `cargo test -p ashiato-server st59_web_password_handler_rejects_non_web_callers` rc=0
- `cargo test -p ashiato-server st59_web_password_handler_applies_all_five_validation_rules` rc=0
- `cargo test -p ashiato-server st59_web_password_handler_uses_new_password_after_rebuilding_app` rc=0
- `cargo test -p ashiato-server st59_both_change_values_are_not_logged` rc=0
- `tools/release/install-check.sh --self-test` rc=0
