# CH15 settle（final review の R13〜R17。2026-11-11）

- R13: tasks.md の Global Constraints に「通常の路へ昇格した後に触れたパス」（`testdb.rs`・`st55_tests.rs`）を足し、照らす形の App の参照を `testdb::verifier_app` に変えた。design.md の「最後の段の処置で足したこと」に 1 行
- R14: `web_session.rs` の `run_to_completion` の `JoinError` の枝に `tracing::error!(kind = "join", op = "login.check", …)`（`panicked` の項目は tools/check-log-private.sh の許可の外なので載せない）。design.md に 1 行
- R15: コードは変えず、design.md に「取り消された成功のログインは使われない印の行を残す」を書いた（受け入れ）
- R16: `check_given_password(app, body: Bytes)` に変え、断る数の判定の後に `given_password(&body)` → `drop(body)` → 印を待つ。2 つの呼び手は本文を move で渡す
- R17: 照合の本体を `pub(crate) async fn check_with_permit(app, given, permit)` に分けた。試験 `ch15_verifier_check_runs_off_the_async_worker` は `hold_login_check_for_test()` で印を取ってから `check_with_permit` を 1 回 poll する

走らせたもの:
- `scripts/quiet-run ct -- cargo test -p ashiato-server -- ch15_ st55_ st56_ web_session`（.env 読み）: rc=0。lib 86 passed・別の target 4 passed（log `.superpowers/sdd/logs/20261111-082136-ct.log`。`panicked` を外した後に同じコマンドを走らせ直して 86 passed（`.superpowers/sdd/logs/20261111-082414-ct.log`））
- `cargo clippy -p ashiato-server --all-targets -- -D warnings`: rc=0
- 変異 D2（`check_with_permit` の `spawn_blocking` を外し、async の worker で `accepts`）: `cargo test -p ashiato-server --lib ch15_verifier_check_runs_off` が `1 failed`。`web_session.rs` を戻して上の緑
- `python3 scripts/review_triage.py . ch15-collector-setup-verifier-password --no-verdict`: rc=0
- `tools/check-log-private.sh`: rc=0
