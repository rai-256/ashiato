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

## settle（final review の後）: R22〜R27

### R22（fixed 9.1）
- `crates/server/src/lib.rs`: 読み手の無い `request_secrets` と task_local の `REQUEST_SECRETS` を消した。`replace_secrets` の要らなくなった `#[allow(dead_code)]` を外した
- `crates/server/src/web_session.rs`: `refresh_cookie` の `REQUEST_SECRETS.scope(...)` を外し（読み手が無い）、呼び手の無い `check_given_password_with_secrets` を消した
- `cargo clippy -p ashiato-server --all-targets -- -D warnings` → rc=0（Finished。警告なし）。9.1 の全体は `scripts/verify-run 9.1`（結果は下の「検証」）

### R23（escalated。コードは仕様の順に戻した）
- 指摘は kind=premise・loss=uncaptured。`review_triage.py` は premise を `fixed` で閉じさせない（仮の D13 を足して試したが「kind=premise は人間に返すもの」で FAIL）。そこで `deep.md` 第 2 回 Q4（未回答）・`deep-questions-r2.json`・`docs/briefs/ST59-deep-r2.html` で本人に返した
- コードは推奨の側（仕様の本文・design D1/D9 の順）にした: `/web-password` は 照合 → ハッシュ → 衝突の検査 → 新しい tag の印の行 → `ok`/204 の行 → 書き戻し → 差し替え → `Set-Cookie`。`/collector-token` は 照合 → 値を作る → `ok`/200 の行 → 書き戻し → 差し替え。印の行か `ok` の行を書けなければ何も変えずに 500。書き戻しに落ちたら差し替えず、`forbidden`/500 の打ち消しの行を足して 500 `not_saved`
- 書き戻しは一時のファイルからの名前の付け替え（D2）なので、巻き戻しは要らなくなった。`restore_setting_after_failed_change`・`service::replace_setting_with_previous`・`service::restore_setting` を消した（R15・R16・R20 の巻き戻しの道はもう無い）
- 試験 `st59_both_write_failure_returns_not_saved`（印「書き戻しに落ちた求めは、変えた行と打ち消しの行の 2 行で残る」）: `ok` の行も `old + 1` を見るように戻し、口ごとに `ok` の行の `max(at)` ≤ 打ち消しの行の `max(at)`（打ち消しが後に並ぶ）を足した
- 試験 `st59_both_access_log_failure_returns_500_without_changes`: `ok` の行を書けない場合は書き戻しが起きないので、`server.json` の比べを JSON からバイトに戻した（R17 の緩めを戻した）
- 本人が「後に書く」を選んだら、答えの後の見直しで順と spec を直す

### R24（fixed 6.1）
- `web/src/messages.ts` の `secretChange.setupLead` を spec の案内の全文「全部の収集アプリ（携帯と PC）にこの文字列を貼り直してください。携帯に入れる文字列は、携帯から開いた画面で『収集アプリの設定』から出してください」にした
- `web/e2e/st59-change-secrets.spec.ts`: 案内を全文の `getByText(..., { exact: true })` で見るようにした（部分一致をやめた）

### R25（fixed 9.1）
- `crates/server/src/st55_tests.rs`: `env_lock()` を `pub(crate)` にした（鍵は `ENV_LOCK` の 1 つ）
- `crates/server/src/st59_tests.rs`: `ST59_SETTINGS_LOCK` を消し、`st59_service_environment` と開発の形の 2 本の試験は `super::st55_tests::env_lock()` を取る。手で環境変数を戻していた 3 か所を、drop で戻す guard `EnvRemoved::remove(&SERVICE_ENV)` にした（assert が落ちても戻る）

### R26（fixed D1）
- `web_password_post_inner`・`collector_token_post_checked`: 照合の後の印を `permit` に受け、`Throttled`・`Mismatch` の枝の最初で `drop(permit)` してから行を書き、失敗の遅れを待つ（ログインの口と同じ。失敗は blocking task の中で数え終えている）。通った枝は今どおり差し替えの後まで印を持つ（D1）

### R27（fixed D9）
- 変える 2 つの口の照合の後の 500 の道（ハッシュ・衝突の検査・印の行の INSERT・値を作る task・書き戻し。JoinError も含む）は、どれも `tracing::warn!(op = …, reason = %…)`（理由は秘密を含まない語。`SettingsWriteError` の Display・JoinError・`collides`）を出し、`forbidden`/500 の行を書いてから返す。印の行の INSERT は今どおり `internal_at` が sqlstate を出す
- 試験 `st59_both_write_failure_returns_not_saved` にログの取り込みを足し、`web_password.save`・`collector_token.save` と `settings_write_failed` が 2 回出ることを見る

### 検証
- `cargo fmt -p ashiato-server`・`cargo clippy -p ashiato-server --all-targets -- -D warnings` → rc=0
- `cargo test -p ashiato-server --lib st59_ -- --list` → 39 本。`scripts/quiet-run st59 -- cargo test -p ashiato-server st59_` → rc=0（`39 passed; 0 failed`、67s）
- `scripts/quiet-run st55 -- cargo test -p ashiato-server --lib st55_` → rc=0（`43 passed; 0 failed`）
- `scripts/verify-run 6.1` → PASS（e2e `st59-change-secrets.spec.ts` 9 passed・210s / `tsc && vitest && lint` rc=0・42s）
- `scripts/verify-run 9.1` → PASS（fmt・clippy の linux と windows-gnu・`cargo test --workspace` rc=0・648s / `check-openapi.sh`・`check-log-private.sh`・`check-secrets.sh`・`check_scenarios.py` rc=0・72s）。R22 の「HEAD で 9.1 の全体を取り直す」はこれ
- `python3 scripts/review_triage.py . st59-change-password-from-web --no-verdict` → `triage: OK` rc=0

## 答えの後の見直し（2026-11-23）

### R23（escalated のまま。答えに沿って処置の行を書き直した）
- deep.md 第 2 回 Q4 の答え（`deep-answers-3.txt`）: 本人が仕様どおりの順（`ok` の行 → 書き戻し → 落ちたら打ち消しの `forbidden`/500 の行）を選んだ。推奨の側と同じ
- コードは前の回（04cfe42dd）でこの順に戻してある。`git diff --stat 04cfe42dd HEAD -- crates web` は空（その後の commit は文書だけ）。spec・design D9 もこの順のままで、直すものは無い
- `review_triage.py` は kind=premise・loss=uncaptured を `escalated` 以外で閉じさせないので、動詞は `escalated` のまま、処置の行に答えと効く先を書いた
- 検証: `cargo test -p ashiato-server st59_both_ -- --list` で 8 本（`st59_both_write_failure_returns_not_saved`・`st59_both_access_log_failure_returns_500_without_changes` を含む）。`scripts/quiet-run st59-r23 -- cargo test -p ashiato-server st59_both_` → rc=0、`test result: ok. 8 passed; 0 failed`（`.superpowers/sdd/logs/20261123-183328-st59-r23.log`）

## final review 2 回目の指摘（2026-11-23）

### R28（fixed D9）
- `web_session.rs` の 2 つの口の書き戻しの失敗の道で、打ち消しの行の書き込みを `app.access.write(failed()).await?` から `let _ = app.access.write(failed()).await;` にした。行を書けなくても `not_saved` の 500 を返す。書けなかった理由は `AccessLog::write` の `internal_at("access_log.insert", …)` が sqlstate つきで error のログに出す（新しい warn は足していない）。書き戻しの理由の warn（`*.save`）は今どおり先に出る
- 試験 `st59_both_write_failure_returns_not_saved_even_if_refusal_row_fails`（`FailingRefusalSink`: `forbidden`/500 の行だけ書けない記録の口 + `server.json.writing` の dir で書き戻しを落とす）。2 つの口とも 500・本文 `{"error":"not_saved"}`、設定のファイルはバイトのまま
- 壊して確かめた: `/web-password` 側を `?` に戻すと、この試験が `response_body_json` で落ちる（`test result: FAILED. 0 passed; 1 failed`）。戻して通る

### R29（fixed D9）
- 起こせる道（印の行の INSERT の失敗）の試験 `st59_web_password_session_insert_failure_is_logged_and_recorded` を足した。使い捨ての DB（`testdb::isolated_with_app()`）でログインした後に `core.web_session` へ BEFORE INSERT で例外を投げる trigger を張り、`/web-password` が 500・ログに `web_password.session_insert`・記録に `forbidden`/500 が 1 行・`ok`/204 が 0 行・設定のファイルは変わらない、を見る
- 起こせない道（argon2 の hash・収集アプリの値の生成・衝突の検査の `Ok(false)`）は、理由を試験の頭のコメントに書いた。どれも R30 の `refuse_internal` を通るので、行とログの形は同じ

### R30（fixed D9）
- `joined()`（`spawn_blocking` の `Result<Result<T,E>,JoinError>` → `Result<T,String>`）と `refuse_internal()`（warn・`forbidden`/500 の行・500 を 1 か所で）を足し、hash・衝突の検査・値の生成の 4 つの分岐を 1 つずつにした。`not_saved_reason` は `joined` に置き換えて消した。warn の文言は 2 つの口で共通の「変える求めを 500 で断る」に、口は `op` で分かれる

### 検証
- `cargo fmt -p ashiato-server --check` rc=0・`scripts/quiet-run st59-clippy -- cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0
- `scripts/quiet-run st59 -- cargo test -p ashiato-server st59_` → rc=0、`test result: ok. 41 passed; 0 failed`（新しい 2 本を含む。`.superpowers/sdd/logs/20261123-184155-st59.log`）
