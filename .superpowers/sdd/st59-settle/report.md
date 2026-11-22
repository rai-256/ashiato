# settle 2026-11-22

- R1/R8/R12: permit 取得後に秘密組を取得し、permit を blocking 検証へ所有移動した。`cargo test -p ashiato-server st59_both_`: 8 passed。
- R7: install-check が合言葉変更応答の Cookie jar を保存するよう変更した。
- R9/R10/R13: 成功監査行を設定保存後にし、CPU/ファイル操作を blocking 側へ移し、throttle の早期判定を追加した。`cargo test -p ashiato-server st59_both_`: 8 passed。
- R11: 一時ファイルへ元の Unix mode を引き継ぎ、失敗時に削除するよう変更した。`cargo check -p ashiato-server`: rc=0。
# settle 再々処置（2026-11-22）

- R2: ログイン照合後・core.web_session の INSERT 直前を止める test-only の同期点を追加した。multi-thread の Scenario 試験で、ここに実際の POST /web-password を割り込ませ、照合時の古い tag の cookie が変更後に 401 となることを確認する。
- R18: Netscape cookie jar の行末は Python の改行文字だけを除く rstrip に直し、値末尾の n やバックスラッシュを削らないようにした。
- R19: blocking 照合の JoinError を panic にせず不一致として失敗数へ反映し、permit は失敗数を記録するまで外側で保持するようにした。
- R20: 設定の巻き戻し失敗を捨てず、秘密やパスを含まない key・理由を error log に残す共通処理にした。

検証:

- cargo fmt --all -- --check rc=0
- cargo test -p ashiato-server st59_swap_login_cookie_uses_the_checked_tag_and_is_revoked_after_swap rc=0（1 passed）
- cargo test -p ashiato-server st59_ rc=0（38 passed）
- tools/release/install-check.sh --self-test rc=0

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

# settle 再々々処置（2026-11-22）

- R2: re-review 2 は 8e0614028 を見ていた。その後の 390bdc385 で、Scenario の印つき・`multi_thread` の `st59_swap_login_cookie_uses_the_checked_tag_and_is_revoked_after_swap`（st59_tests.rs:1348）を足し、ログインを差し込み口 `wait_before_cookie_insert_for_test`（web_session.rs:1530。照合の後・`core.web_session` の INSERT の前）で止めて、本物の `POST /web-password` を通している（旧名の単一 thread の試験は置き換え済み）。この回はコードを変えず、根拠の変異で落ちることを確かめた。

検証:

- `cargo test -p ashiato-server st59_swap_login_cookie -- --list` → 1 本
- 現在の木: `cargo test -p ashiato-server st59_swap_login_cookie_uses_the_checked_tag_and_is_revoked_after_swap` rc=0（1 passed）
- 変異（web_session.rs:1535 `.bind(secrets.tag())` → `.bind(app.secrets_snapshot().tag().to_vec())`）: 同じ試験が rc=101、`st59_tests.rs:1372` で `assertion left == right failed  left: 200 right: 401`。変異は戻した（`git checkout`）
- `cargo test -p ashiato-server st59_` rc=0（38 passed）

## settle（re-review 3 の後）: R12・R21

### 何を変えたか
- `crates/server/src/web_session.rs` の `check_with_permit_using_secrets`: 印（`LoginCheckPermit`）と `Arc<LoginLimiter>` を `spawn_blocking` の closure に所有ごと渡し直した。closure の中で照合し（`catch_unwind` で照合の panic も失敗に数える。R19）、失敗なら `record_failure()` してから印を返す。呼び出し元の future が await の途中で捨てられても、task は最後まで走り、数えた後で印を放す（review R78 の保証を戻した）。
- 返す印は `Option<LoginCheckPermit>`。照合が通ったときは呼び出し元が差し替えの終わりまで持つ（R14 の形は保つ）。`JoinError`（task が走らずにランタイムの終わりで取り消された）のときは外側で `record_failure()` し `(Mismatch, None)`。
- 試験の口 `recorded_failures_for_test(app)`（`#[cfg(test)]`）を足した。

### 試験
- 新しい試験 `web_session_tests::ch15::st59_dropped_check_counts_failure_before_releasing_permit`: 印を取って `check_with_permit` を 1 回 poll（blocking の照合が始まる）→ future を捨てる → 次の印を取れた時点で失敗が 1 件数えられていることを確かめる。
- 変異: 印を外側の future に置く旧い形（re-review 3 が指した 1447 行の形）に戻すと、この試験は `left: 0 right: 1` で落ちた（`.superpowers/sdd/logs/*-mut.log`）。戻して通る。
- `cargo test -p ashiato-server st59_dropped_check`: 1 passed（`-- --list` 相当で 1 本あることをログで確認）。
- `cargo test -p ashiato-server st59_`: 39 passed。`ch15`: 9 passed。`web_session`: 37 passed。
- `cargo test -p ashiato-server`（全体）: 2035 passed・1 failed。落ちたのは `ch15_verifier_cancelled_requests_still_leave_a_row`（5 秒の待ちの切れ。load average 28 の下）。単独で 3 回走らせて 3 回とも ok。同じ試験は今日 08:43（この変更の前）のログ `20261122-084339-web-session.log` でも落ちており、既存の負荷による揺れ。
- `cargo fmt -p ashiato-server` を当てた。`cargo clippy -p ashiato-server --all-targets -- -D warnings` は `request_secrets`（lib.rs:1000）と `check_given_password_with_secrets`（web_session.rs:1414）の dead_code で落ちる。どちらもこの回の diff が触れていない関数で、この回の処置の範囲外（報告だけ）。
