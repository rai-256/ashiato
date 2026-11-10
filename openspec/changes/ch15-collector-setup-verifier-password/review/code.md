<!-- 処置の判定: re-review -->

# CH15 code_verify（2026-11-10）

対象: base `fec8c2674` → HEAD `08c738b18`（本番のコードの差分は `crates/server/src/web_session.rs` だけ。試験は `web_session_tests.rs` の `mod ch15`）。

## 申告: 2/2（`[x]` 1.1・1.2。9.1 は人間の確認待ちで `[ ]`）・検証コマンド一覧

| 項目 | 検証コマンド（tasks.md） | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server ch15_'` rc=0 | rc=0。3 passed / 1949 filtered out（`ch15_verifier_collector_setup_returns_token`・`…_wrong_password_is_forbidden`・`…_shares_login_failure_limit`） | 一致 |
| 1.1（直す前） | evidence.jsonl の FAIL（head `edf97de6a`・exit 101） | ログ `20261110-220532-evidence-1.1-…log`: `returns_token` と `wrong_password_is_forbidden` が `left: 403 / right: 200` で FAILED、`shares_login_failure_limit` は ok。組み立ての失敗ではない | 一致 |
| 1.2 | `ch15_ -- --list` が 3 本以上 | 3 本 | 一致 |
| 1.2 | `st56_ -- --list` が 7 本以上 && `cargo test st56_` | 7 本・rc=0（7 passed） | 一致 |
| 1.2 | `cargo test st55_ && cargo test web_session` | rc=0（43 passed・31 passed） | 一致 |
| 1.2 | `! grep -n "login\.password\b" web_session.rs` | rc=0（当たり無し） | 一致 |
| 1.2 | `.accepts(` が 1・`LOGIN_CHECK` が 2（コメント行を除く） | accepts=1 LOGIN_CHECK=2 | 一致 |
| 1.2 | `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st56-collector-setup.spec.ts'` | rc=0・4 passed（41.6s） | 一致 |
| 1.2 | `cargo clippy -p ashiato-server --all-targets -- -D warnings` | rc=0 | 一致 |
| 1.2 | `./tools/check-openapi.sh` | rc=0（「API の契約 OK」） | 一致 |

実測: 申告と一致した。検証コマンドは全部実在し、rc=0 だった。`lane-verdict.json` の red→green の根拠も、上のログで中身まで確かめた。
ただし、下の R1〜R3 のとおり、**検証が緑のまま壊せる振る舞いが 3 つ**ある。

変異の確かめ方（R2・R3 で共通）: 作業ツリーの `web_session.rs` を 1 か所だけ壊し、1.2 の検査のうち本番のコードで結果が変わりうるもの（grep の数え・`cargo test -p ashiato-server -- ch15_ st56_ st55_ web_session`（81 本）・`cargo clippy -p ashiato-server --all-targets -- -D warnings`）を走らせ、毎回 `git checkout` で戻した。openapi と e2e は、口の型と合言葉そのものの形の画面しか見ないので、変異には当てていない。

## R1. CH15 で、資格情報の口に「取り消された求めは失敗に数えるのに、読み出しの記録に行を残さない」窓ができた
- 成果物: crates/server/src/web_session.rs（`check_given_password` :647-685・`collector_setup_post` :579-608）
- 根拠: 一時的な試験を `mod ch15` に足して走らせた。手順は、照らす形の App でログインし、違う合言葉の `POST <口>` を `tokio::time::timeout(20ms)` で取り消し、3 秒待ってから資格情報の口に違う合言葉を 9 回送り、最後に正しい合言葉を送る。数えたのは `core.access_log` の、その口の `login_ok` 以外の行:
  - HEAD・`/collector-setup`: `timed_out=true`・取り消し後の行 0・最後の正しい合言葉は 429。取り消された 1 回は blocking の側で `record_failure()` まで進み、10 回目の失敗に数えられた。それなのに行は残っていない
  - base（`git checkout fec8c2674 -- crates/server/src/web_session.rs`）・`/collector-setup`: `timed_out=false`・行 1・最後は 429。base は 20ms 以内に答え、行も残った
  - HEAD・`/session`: `timed_out=true`・行 0・最後は 429。base の `/session` も同じく `timed_out=true`・行 0 だった。ログインの口は ST55 からこの窓を持っていて、CH15 はそれを共通の関数ごと資格情報の口に広げた

  正典の data-sensitivity spec.md:203 は「ログイン（成功・失敗・一時的な断り）…を、1 件ごとに読み出しの記録に 1 行残す」と言う。CH15.md の壊してはいけないもの 3 は「この口の行が成功も失敗も 1 行ずつ残り」と言う。記録の書き込みの前に照合の `spawn_blocking` を待つので、照らす形では Argon2id の時間がそのまま窓になる。共通の関数のコメント（:663-665、R78）は「接続が切れてこの future が捨てられても」と、future が捨てられることを前提に、数え上げだけを守っている。落ちた行は後から作り直せない。ただ、直し方は技術の選択で、失う側を選ぶ問いではないので `technical` にした。試験は戻した（`git status --porcelain` の crates 分は前後とも空、`git diff -- crates` は 0 バイト）
- kind: technical
- 提案: 照合から記録の書き込みまでを、取り消されない側で行う。例: 2 つの口の「照合 → 行を書く」を `tokio::spawn` した task の中で走らせ、handler はその `JoinHandle` を待つだけにする（spawn した task は、待つ側が捨てられても走り切る）。上の手順（timeout で取り消し → 行の数）を、2 つの口の `ch15_` の試験として残す

## R2. D1 の要（印を照合の終わりまで持つ・照合を blocking の側で行う）を壊しても、1.1・1.2 の検証は通る
- 成果物: crates/server/src/web_session.rs（`check_given_password`）/ tasks.md 1.2 の grep の検査 / crates/server/src/web_session_tests.rs
- 根拠: 変異を 1 つずつ当てた結果:
  - 変異 A: `drop(permit)` を `spawn_blocking` の前へ移した（印を照合の前に放す。ST55 review R23・R78 の穴そのもの）。結果は grep `accepts=1 LOGIN_CHECK=2`、試験 rc=0（81 passed）、clippy rc=0
  - 変異 D: 最初は閉包をその場で呼ぶ形で壊したが、clippy rc=101（`try not to call a closure in the expression where it is declared`）で止まった。そこで、`spawn_blocking(move || {…}).await.unwrap_or_else(…)` を同じ中身のブロック `{ … }` に置き換え、Argon2id を async の worker の上で回す形にした（D2）。結果は grep `accepts=1 LOGIN_CHECK=2`、試験 rc=0（81 passed）、clippy rc=0
  - 揺れないこと: spec review R3 の処置で足した grep（`.accepts(` 1 か所・`LOGIN_CHECK` 2 か所）は、どちらの変異も止めない。R3 の処置は「並べて送る試験は時刻に依る揺れを持ち込むので足さない」としたが、実測では揺れなかった。一時的な試験（照らす形で 9 回失敗させ、`tokio::join!` で違う合言葉を 2 本同時に送り、どちらかが 429 であることを見る。`multi_thread`・worker 4）は、今の実装で 3/3 が `a=403 b=429`（rc=0）、変異 A で 3/3 が `a=403 b=403`（rc=101）だった。試験は戻した（`git diff -- crates` は 0 バイト）
- kind: technical
- 提案: 上の並べて送る試験を `ch15_` に足す（変異 A を落とす）。変異 D2 は数えでは見えない。`current_thread` の runtime で照合の最中に別の task が進むことを見る試験を足すか、落とせないなら tasks に「blocking の側」は grep でなく目視の検査だと書き分ける

## R3. 資格情報の口から「空の合言葉を失敗に数える」と「失敗の遅れ」を消しても、検証は通る
- 成果物: crates/server/src/web_session.rs（`check_given_password` :659-662・`collector_setup_post` :601）/ crates/server/src/web_session_tests.rs
- 根拠: 変異を 1 つずつ当てた結果:
  - 変異 C: `if given.is_empty() {`（:659）の直後の `app.login.limiter.record_failure();` を消した。結果は試験 rc=0（81 passed）、clippy rc=0、grep `accepts=1 LOGIN_CHECK=2`
  - 変異 B: `collector_setup_post` の不一致の枝の `tokio::time::sleep(app.login.failure_delay).await;`（:601）を消した。結果は試験 rc=0（81 passed）、clippy rc=0、grep `accepts=1 LOGIN_CHECK=2`
  - 試験の範囲: design D1 は「数えの下では空の合言葉は失敗に数えて、資格情報の口は 403」と言う。CH15.md の壊してはいけないもの 2 は「失敗の遅れと、ログインの口と共有する失敗の数え」を挙げる。`ch15_verifier_collector_setup_wrong_password_is_forbidden` は空の合言葉の 403 と本文だけを見て、数えたかを見ない。`shares_login_failure_limit` の失敗は、どれも空でない合言葉。遅れを見る試験は `web_session_tests.rs:270`（ログインの口）だけ。`grep -rn failure_delay crates/server/src --include=*_tests.rs` で `Duration::ZERO` 以外に当たるのはこの 1 か所で、資格情報の口の遅れはどの試験も見ない。遅れの穴は base からあるが、CH15 が壊してはいけないものに挙げた以上、守る試験が要る
- kind: technical
- 提案: `ch15_` に 2 本足す。(1) 照らす形で空の合言葉を 9 回と、ログインの口で 1 回失敗させた後、正しい合言葉が 429 になる（変異 C を落とす）。(2) `failure_delay` を 300ms にした App で資格情報の口に違う合言葉を送り、応答まで 300ms 以上かかる（`web_session_tests.rs:270` と同じ形。変異 B を落とす）

---

## 手 1: 該当なし
確かめた範囲: `ch15_` の 3 本に、実装の出力を写した固定値は無い。比べる値は、試験の中で決めた `API_TOKEN` と `{"error": "wrong_password"}` だけ。照らす値は、試験の中で `password_verifier_for_test`（web_session.rs:184）が毎回作る。その作り方は `Argon2::default()` の `hash_password` で、インストーラ（crates/installer/src/password.rs:56）と同じ。python の argon2 は入っていない（`ModuleNotFoundError`）ので、別の実装での照合はしていない。

## 手 2: R2・R3
本番のコードの変異 A・B・C・D2 は、grep・試験・clippy を全部すり抜けた。最初の D は clippy で止まった。base の木で `ch15_` が落ちること（直す前の FAIL）はログで確かめた。組み立ての失敗ではなく、403≠200 の不一致だった。

## 手 3: 該当なし
確かめた範囲: `python3 scripts/check_scenarios.py . ch15-collector-setup-verifier-password` は `scenarios: OK`（warn は、この change の外にある既存の印だけ）。印の付いた 3 本が観測するもの:
- `returns_token`: 200・`Cache-Control: no-store`・本文 `{"token": API_TOKEN}`
- `wrong_password_is_forbidden`: 1 文字違いと空の合言葉で 403・本文・資格情報を含まない・その後の正しい合言葉で 200（ログインが保たれる）
- `shares_login_failure_limit`: 9+1 回の失敗の後、正しい合言葉と空の合言葉が 429

画面の階層（文字列とコピーのボタン）は、合言葉そのものの形のサーバで e2e `st56-collector-setup.spec.ts` が見ている（4 passed）。照らす形の画面は 9.1 に置いてある（design の Risks に書いた分担どおり）。Scenario の主張の階層との食い違いは見つからなかった。数えの共有について、ch15 が見るのは「資格情報の口の失敗 → ログインの口の失敗 → 資格情報の口が 429」の向きだけ。

## 手 4: 該当なし
確かめた範囲: deep.md は軽い路の申告の写しで、「本人の答え」が無い。CH15 は、design が触れる数（失敗の上限・遅れ）を変えていない。
- 上限: `THROTTLE_MAX_FAILURES`（web_session.rs:48）を 10→11 に書き換えたら、`ch15_verifier_collector_setup_shares_login_failure_limit` が FAILED になった（rc=101。戻した）
- 既定の遅れ 1 秒: `login_failure_delay_default_is_1_second`（web_session.rs:913）がある。これは読んだだけで、書き換えて落ちるかは試していない
- 資格情報の口の遅れが固定されていないことは R3 に書いた

## 手 5: 該当なし
確かめた範囲: `[x]` は 1.1・1.2 の 2 件。両方の検証コマンドは実在し、上の表のとおり全部 rc=0。`cargo test <名前>` 型は、`-- --list` で 3 本（ch15_）・7 本（st56_）あった。st55_ は 43 本、web_session は 31 本が走った。`cargo test --test <名前>` 型は無い。`[ ]` の 9.1 は物理（realdata）で、人間の確認待ち。

## 手 6: R1
取り消された求めで記録の行が落ちる。ほかに確かめた範囲:
- `spawn_blocking` の側が落ちたときは、失敗に数えて不一致になる（:676-679。base の `session_post` と同じ形。読んだだけ）
- 失敗の数え（`LoginLimiter`）は `Mutex<VecDeque<Instant>>`（web_session.rs:257）で、DB に持たない。CH15 はここを変えていない（読んだだけ。再起動は走らせていない）
- 照らす形で空の合言葉は、Argon2id を回さず不一致になる。base の `session_post` は照らす形で空でも Argon2id を回していたが、結果は同じ不一致（読んだだけ）

## 手 7: 該当なし
`python3 scripts/plan_fix.py pending openspec/changes/ch15-collector-setup-verifier-password` の結果は `[]`。

作業ツリー: 壊したもの・足したものは全部戻した。`crates/` の `git status --porcelain` と `git diff` は、検証の前後とも空。この席が変えたのは `review/code.md` だけ。
