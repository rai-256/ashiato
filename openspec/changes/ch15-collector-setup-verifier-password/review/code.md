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
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R2. D1 の要（印を照合の終わりまで持つ・照合を blocking の側で行う）を壊しても、1.1・1.2 の検証は通る
- 成果物: crates/server/src/web_session.rs（`check_given_password`）/ tasks.md 1.2 の grep の検査 / crates/server/src/web_session_tests.rs
- 根拠: 変異を 1 つずつ当てた結果:
  - 変異 A: `drop(permit)` を `spawn_blocking` の前へ移した（印を照合の前に放す。ST55 review R23・R78 の穴そのもの）。結果は grep `accepts=1 LOGIN_CHECK=2`、試験 rc=0（81 passed）、clippy rc=0
  - 変異 D: 最初は閉包をその場で呼ぶ形で壊したが、clippy rc=101（`try not to call a closure in the expression where it is declared`）で止まった。そこで、`spawn_blocking(move || {…}).await.unwrap_or_else(…)` を同じ中身のブロック `{ … }` に置き換え、Argon2id を async の worker の上で回す形にした（D2）。結果は grep `accepts=1 LOGIN_CHECK=2`、試験 rc=0（81 passed）、clippy rc=0
  - 揺れないこと: spec review R3 の処置で足した grep（`.accepts(` 1 か所・`LOGIN_CHECK` 2 か所）は、どちらの変異も止めない。R3 の処置は「並べて送る試験は時刻に依る揺れを持ち込むので足さない」としたが、実測では揺れなかった。一時的な試験（照らす形で 9 回失敗させ、`tokio::join!` で違う合言葉を 2 本同時に送り、どちらかが 429 であることを見る。`multi_thread`・worker 4）は、今の実装で 3/3 が `a=403 b=429`（rc=0）、変異 A で 3/3 が `a=403 b=403`（rc=101）だった。試験は戻した（`git diff -- crates` は 0 バイト）
- kind: technical
- 提案: 上の並べて送る試験を `ch15_` に足す（変異 A を落とす）。変異 D2 は数えでは見えない。`current_thread` の runtime で照合の最中に別の task が進むことを見る試験を足すか、落とせないなら tasks に「blocking の側」は grep でなく目視の検査だと書き分ける
- 処置: fixed 1.2
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R3. 資格情報の口から「空の合言葉を失敗に数える」と「失敗の遅れ」を消しても、検証は通る
- 成果物: crates/server/src/web_session.rs（`check_given_password` :659-662・`collector_setup_post` :601）/ crates/server/src/web_session_tests.rs
- 根拠: 変異を 1 つずつ当てた結果:
  - 変異 C: `if given.is_empty() {`（:659）の直後の `app.login.limiter.record_failure();` を消した。結果は試験 rc=0（81 passed）、clippy rc=0、grep `accepts=1 LOGIN_CHECK=2`
  - 変異 B: `collector_setup_post` の不一致の枝の `tokio::time::sleep(app.login.failure_delay).await;`（:601）を消した。結果は試験 rc=0（81 passed）、clippy rc=0、grep `accepts=1 LOGIN_CHECK=2`
  - 試験の範囲: design D1 は「数えの下では空の合言葉は失敗に数えて、資格情報の口は 403」と言う。CH15.md の壊してはいけないもの 2 は「失敗の遅れと、ログインの口と共有する失敗の数え」を挙げる。`ch15_verifier_collector_setup_wrong_password_is_forbidden` は空の合言葉の 403 と本文だけを見て、数えたかを見ない。`shares_login_failure_limit` の失敗は、どれも空でない合言葉。遅れを見る試験は `web_session_tests.rs:270`（ログインの口）だけ。`grep -rn failure_delay crates/server/src --include=*_tests.rs` で `Duration::ZERO` 以外に当たるのはこの 1 か所で、資格情報の口の遅れはどの試験も見ない。遅れの穴は base からあるが、CH15 が壊してはいけないものに挙げた以上、守る試験が要る
- kind: technical
- 提案: `ch15_` に 2 本足す。(1) 照らす形で空の合言葉を 9 回と、ログインの口で 1 回失敗させた後、正しい合言葉が 429 になる（変異 C を落とす）。(2) `failure_delay` を 300ms にした App で資格情報の口に違う合言葉を送り、応答まで 300ms 以上かかる（`web_session_tests.rs:270` と同じ形。変異 B を落とす）
- 処置: fixed 1.2

---
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

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

## security review（fec8c26..d4e77c2）

見た経路:
- 新しく届く経路は 1 つだけ: 照らす形（配った形）のサーバで `POST /collector-setup` が収集アプリの token を返すようになった。base では照らす形の口はどの合言葉も 403 だった（proposal Why）。router への足し・新しい listen・接続先・依存（`Cargo.toml` / `Cargo.lock` / `web/package*.json`）の変更は無い（`git diff --first-parent fec8c26..HEAD --stat` は `web_session.rs`・`web_session_tests.rs` と change の文書だけ）（3）
- 入口 → `crate::decided`（web_session.rs:533）→ `caller` が無ければ 401（:542-551）→ `Caller::WebSession` でなければ 403（:552-562）→ JSON でなければ 415（:564-577）→ ここで初めて `check_given_password`（:579-580）。CH15 は照合より前の 3 つの関門に触れていない（`git diff` の hunk は :579 から）（4）
- `check_given_password`（:647-685）→ `LOGIN_CHECK` の印 → `throttled()` → 空なら失敗に数えて不一致 → `spawn_blocking` で `WebLogin::accepts`（:348）→ 照らす形は `verify_password_verifier`（:164。`PasswordHash::new` が読めなければ false）。一致のときだけ :610-620 で token を返す（4）
- 本文の値（`password`）は `given_password`（:635-643）で `serde_json::Value` に読んで文字列だけを取る。SQL・パス・シェル・HTML には入らず、Argon2id の照合にだけ渡る。文字列でない・欠ける・JSON でないは空 → 403（4）
- 出力: 読み出しの記録の行は `collector_setup_entry`（:499-512）の固定の項目（caller・via・route・method・outcome・status）だけで、本文を持たない。`check_given_password` は tracing を出さない。`session_post` の tracing は `kind` と固定の文だけで、CH15 で変わらない（4）
- 失敗の数えは 2 つの口で同じ `app.login.limiter` を共有し、資格情報の口がログインの口より緩い合言葉の当て先にならない（`ch15_verifier_collector_setup_shares_login_failure_limit` が 9+1 回の後の正しい合言葉を 429 と見る。code_verify の表で rc=0）（4）
- 取り消された求めで記録の行が落ちる窓（監査の跡の欠け）は R1 にもう書いてあるので、ここで重ねない（—）

安全の決め手: **資格情報の口が token を返すのは、`Caller::WebSession`（画面の有効なログインの印）・JSON の本文・`check_given_password` の `Match` をすべて通ったときだけで、CH15 は前の 2 つの関門に触れず、照合を `WebLogin::accepts`（ログインの口と同じ照合と失敗の数え）に替えただけ** —— 確かさ 4。
一時的な試験 `ch15_secprobe_gates_in_verifier_mode` を `mod ch15` に足し、照らす形の App（`verifier_app()`）で走らせた（`scripts/quiet-run sec -- cargo test -p ashiato-server ch15_secprobe -- --nocapture`、rc=0・1 passed。ログ `.superpowers/sdd/logs/20261110-225056-sec.log`）:
```
PROBE no_cookie status=401 Unauthorized leaks=false
PROBE bearer status=403 Forbidden leaks=false
PROBE forged_cookie status=401 Unauthorized leaks=false
PROBE text_plain status=415 Unsupported Media Type leaks=false
PROBE verifier_as_password status=403 Forbidden
PROBE body={"password":123} status=403 Forbidden
PROBE body={} status=403 Forbidden
PROBE body={"password":null} status=403 Forbidden
PROBE body=not json status=403 Forbidden
PROBE access_rows=9 leak=false
```
（`leaks` は応答の本文に token が入っていたか。`verifier_as_password` は照らす値そのもの（`$argon2id$…`）を合言葉として送り、照らす値を知る者が通らないことを見た。`access_rows` は `core.access_log` の `/collector-setup` の行で、合言葉・token・`$argon2` のどれも含まない）
試験は戻した（`git status --porcelain crates/` は空・`git diff -- crates` は 0 バイト）。

指摘なし

範囲外（R 番号なし）: `LOGIN_CHECK`（web_session.rs:303）はプロセスで 1 つの印で、ログインの印の要らない `POST /session` に Argon2id を回す求めを並べ続ければ、資格情報の口の照合も待たされる。ST55 からあり、CH15 はこの印を資格情報の口にも通しただけ。

## /code-review（fec8c26..8e451ea）

見た: Claude Code の組み込みの `/code-review`（6 件）。写しは scripts/code_review_record.py

## R4. collector_setup_post now waits for the global LOGIN_CHECK semaphore (an Argon2id check of tens of ms, one at a time) even just to learn it is throttled, so heavy /session traffic delays a logged-in user's setup request even when the reply will only be 429.
- 成果物: crates/server/src/web_session.rs:579
- 根拠: Under a stream of wrong /session attempts, every collector-setup call queues behind each Argon2id check in turn. Before this change it read limiter.throttled() straight away, without the semaphore. A logged-in user's setup request now takes N times the Argon2 time and can time out instead of getting a fast 429.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R5. session_post now parses the JSON body (given_password) before the throttle check; the old code checked throttled() first and parsed only when not throttled.
- 成果物: crates/server/src/web_session.rs:710
- 根拠: While throttled, an attacker sends many /session requests with bodies up to the default 2MB limit. Each body is fully parsed into a serde_json::Value and then thrown away with a 429. The old order skipped this work. Moving the parse after the throttle check, or parsing lazily, avoids it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R6. In plain mode (verifier=None), accepts() is just a constant-time compare, but it still always goes through spawn_blocking, so collector-setup now pays a thread hand-off it did not pay before.
- 成果物: crates/server/src/web_session.rs:670
- 根拠: Every /collector-setup and /session call in plain mode sends a microsecond compare to the blocking pool. That is extra latency and pool pressure. Branching on login.verifier.is_some() would keep the blocking hop only for Argon2id.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: design D1 と tasks.md の Global Constraints が「`spawn_blocking` の側での `WebLogin::accepts`」を 2 つの口の同じ道として決めている（合言葉そのものの形も同じ道に通す。D1「合言葉そのものの形の振る舞いは変わらない」）。合言葉そのものの形は開発・試験の形で、blocking の pool への受け渡しは 1 回の求めに 1 回だけ。DB への書き込み（記録の行）を毎回待つ口なので、受け渡しの遅れは比べて小さい。形ごとに枝を分けると、照らす形だけを async の worker で回す近道（R2 の変異 D2）と見分けにくい枝が増える
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R7. The ch15 test module's login_cookie re-implements testdb::web_cookie (and st56_tests::login_cookie) line for line.
- 成果物: crates/server/src/web_session_tests.rs:467
- 根拠: There are now three copies of the POST /session plus Set-Cookie parsing. If the login contract changes, all three must be updated. Calling testdb::web_cookie(&app, PASSWORD) does the same job.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.2
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R8. verifier_app() repeats the App setup from st55_tests::st55_verifier_login_route_accepts_only_matching_password: the same session key, token, failure_delay=ZERO and App::with_login, all copied.
- 成果物: crates/server/src/web_session_tests.rs:444
- 根拠: Verifier-mode test apps are built in two places with the same magic strings. A shared helper (for example a testdb::verifier_app(password)) would stop them drifting apart when WebLogin construction changes.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.2
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R9. check_given_password returns Result only for a semaphore-closed error (SERVICE_UNAVAILABLE) that cannot happen: LOGIN_CHECK is a static that is never closed. Both callers still have to `?` it.
- 成果物: crates/server/src/web_session.rs:651
- 根拠: This is a dead error path that looks like a real 503 to readers and callers. Returning PasswordCheck directly, with acquire().await.expect(...) or an unreachable comment, makes the contract simpler.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @1fd87dc547653f553602a0748a71fa38adcc0361）
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## settle_review（re-review 1 @1fd87dc）: 処置の直しが新しく壊したもの

## R10. 取り消す時刻を Argon2id 1 回の測定値の半分で決めている。release ビルドで速いと、照合の前の段（約 30ms）より短くなって行が元から無く、偽の赤になりうる。debug（約 530ms）では実害なし。
- 成果物: crates/server/src/web_session_tests.rs（ch15_verifier_cancelled_requests_still_leave_a_row）
- 根拠: crates/server/src/web_session_tests.rs（ch15_verifier_cancelled_requests_still_leave_a_row）（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.2 —— 取り消す時刻を測定値で決めるのをやめた。試験が照合の 1 つずつの印（`LOGIN_CHECK`）を `web_session::hold_login_check_for_test`（`#[cfg(test)]`）で握り、照合を印を待つ所で止めてから 1 秒で取り消す（照合の前の段は約 30ms）。取り消した直後に行が 0 であることも見てから印を放し、5 秒以内に 1 行を見る。release でも PASS、`run_to_completion` の spawn を外す変異で FAILED
- 判定: ADDRESSED（re-review 2 @89ca79431902d67fd502edf3cfeb5c69121b0769）
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## settle_review（re-review 2 @89ca794）: 処置の直しが新しく壊したもの

## R11. 試験が 1 ケースごとに最大約 1 秒、プロセス共通の静的な LOGIN_CHECK を握る。並走する他の verifier の試験の照合はその間待たされる。待つだけで、下限を見る試験は壊れない。全体の cargo test は 1954 passed と報告されていて、実害は観測されていない。
- 成果物: crates/server/src/web_session_tests.rs
- 根拠: crates/server/src/web_session_tests.rs:645-660（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.2 —— 試験が印を握るのを、取り消すまでの 1 秒から「この app の照合が印を待ち始めた合図」が来るまでの短い間に縮めた。合図は `WebLogin` が持つ `#[cfg(test)]` の `Notify`（app ごと。並走する他の試験の照合とは混ざらない）で、`check_given_password` が印を待つ直前に `notify_waiters` する。合図の後すぐに求めの future を drop して取り消し、行が 0 を見てから印を放す。release でも PASS、`run_to_completion` の spawn を外す変異で FAILED
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R12. 照合の前の段（JSON の解析とログインの印の確かめ）が高負荷で 1 秒を超えると、run_to_completion が走る前に取り消されて行が元から無く、偽の赤になりうる。通常の負荷（debug で約 30ms）では起きない。
- 成果物: crates/server/src/web_session_tests.rs
- 根拠: crates/server/src/web_session_tests.rs:645-660（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.2 —— 取り消す時刻を時計で決めるのをやめた。取り消すのは、照合が 1 つずつの印を待ち始めた合図（`WebLogin::check_waiting_for_test`）の後で、そこは `run_to_completion` が照合を spawn した後なので、照合の前の段がどれだけ遅くても行は元から無いことにならない。合図は溜まらない形（`notify_waiters`。cookie を取る前のログインの照合の合図を拾わない）で、求めを出す前に待ち受けを作る。求めは `Box::pin` に入れて drop で実際に取り消す。release でも PASS、`run_to_completion` の spawn を外す変異で FAILED
- 判定: ADDRESSED（re-review 3 @08f7609ac09bb26b9c919090da15c1893fb6b48a）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## final review（eb24379..5d0c6b5）

見た: whole-branch の差分（`.superpowers/sdd/tasks/review-eb243793d..5d0c6b5e4.diff`）を `code-reviewer.md` の席で 1 回読んだ。`ch15_` は 8 passed。CH15.md の壊してはいけないもの 1〜6 は保たれている（失敗の遅れは spawn した task の中で行を書いた後・2 つの口で数えを共有・取り消されても 1 行・tracing に新しい項目なし）。R1〜R12 と重なる指摘は無い。申し送り（`docs/handoff/CH15.md`）は無い。

## R13. 昇格した後も、tasks.md の Global Constraints が実際の差分と食い違ったまま（予定のパスの外の `st55_tests.rs`・`testdb.rs` に触れている）
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（:9・:13）/ openspec/changes/ch15-collector-setup-verifier-password/design.md（「最後の段の処置で足したこと」）
- 根拠: tasks.md:9 は「触ってよいのは lane.json の予定のパスだけ」「`st55_tests.rs` / `st56_tests.rs` … には触れない（触れると通常の路へ昇格する）」、:13 は「照らす形の App は `st55_tests.rs` の … と同じ組み方」と書く。実際の差分は R8 の処置で `crates/server/src/st55_tests.rs`（11 行）と `crates/server/src/testdb.rs`（`verifier_app`・`VERIFIER_APP_TOKEN`、18 行）に触れている。`lane-verdict.json` の `diff.paths` にはこの 2 つが載るが、`promoted.why` は red_green だけを挙げ、パスの逸脱を昇格の理由にしていない。deep.md Q-L2 の「族」が R8 に一言触れるだけで、計画の文書は禁じたまま。後から読む人には、計画が禁じたファイルに触ったように見える（Important）
- kind: technical
- 提案: tasks.md の Global Constraints（または design.md の「最後の段の処置で足したこと」）に、昇格の後に触れたパス（`st55_tests.rs`・`testdb.rs`、R8）を書き足し、:13 の組み方の参照を `testdb::verifier_app` に書き換える。ハーネス側（`lanes.py` の diff の判定が宣言の外のパスを `why` に載せない）は harness2 への申し送り
- 処置: fixed D1 —— tasks.md の Global Constraints に「通常の路へ昇格した後に触れたパス」（`testdb.rs`・`st55_tests.rs`、R8）を 1 項目足し、照らす形の App の組み方の参照を `testdb::verifier_app` に書き換えた。design.md の「最後の段の処置で足したこと」にも 1 行。ハーネス側（`lanes.py` が宣言の外のパスを `promoted.why` に載せない）は harness2 の範囲で、この change では直さない
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R14. 照合の task が panic したとき、tracing にも読み出しの記録にも何も残らずに 500 になる
- 成果物: crates/server/src/web_session.rs（`run_to_completion` の `JoinError` の枝 :614-616）
- 根拠: spawn した task が panic すると handler は 500 を返すが、`tracing::error!` も access_log の行も無い。lib.rs の `internal_at` は内部の失敗を `tracing::error!` で残している。起きる見込みは低いが、「1 件ごとに 1 行」（data-sensitivity spec.md:203）が欠けた理由を後から追えない（Minor）
- kind: technical
- 提案: `JoinError` の枝に `tracing::error!(kind = "join", op = "login.check", …)` を 1 行足す（合言葉・資格情報は載せない）
- 処置: fixed D1 —— `run_to_completion` の `JoinError` の枝で `tracing::error!(kind = "join", op = "login.check", …)` を 1 行残してから 500 にする（合言葉・資格情報は載せない）。design.md に 1 行
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R15. 取り消された正しい合言葉のログインで、cookie の届かない印の行が `core.web_session` に残る
- 成果物: crates/server/src/web_session.rs（`session_post_checked`）/ openspec/changes/ch15-collector-setup-verifier-password/design.md
- 根拠: Q-L2 の `run_to_completion` で、`POST /session` が正しい合言葉のまま取り消されても task が走り切り、`core.web_session` に行を INSERT して `login_ok` の行を書く。cookie は客に届かない。残るのは hash だけで使えないが、期限なし（max_age 0）の既定では消えない。Q-L2 の問いは失敗の側（数えと行）として書かれ、成功の側のこの副作用は design にも deep にも書いていない（Minor）
- kind: technical
- 提案: 受け入れるなら design.md の「最後の段の処置で足したこと」に 1 行書く（取り消された成功のログインは使われない印を 1 行残す）
- 処置: fixed D1 —— 受け入れて design.md の「最後の段の処置で足したこと」に書いた（取り消された成功のログインは、cookie の届かない使われない印の行を `core.web_session` に 1 行残す。残るのは hash だけ。Q-L2 の「走り切る」の成功の側の結果）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R16. 取り消された求めの本文が、印の待ちと失敗の遅れの間も task に残る
- 成果物: crates/server/src/web_session.rs（`collector_setup_checked`・`session_post_checked`）
- 根拠: 本文の `Bytes`（既定の上限約 2MB）が spawn した task に移り、`LOGIN_CHECK` の待ちと失敗の遅れ（既定 1 秒）の間ずっと保たれる。base では取り消しの時点で捨てられていた。失敗 10 回で断る側に入るので量に上限はある（Minor）
- kind: technical
- 提案: 合言葉を取り出した後（`check_given_password` の前後）で本文を `drop` する
- 処置: fixed D1 —— `check_given_password` が本文を `Bytes` で受け取り、断る数の判定の後に合言葉を取り出したら `drop(body)` する。印の待ちと失敗の遅れの間は本文を持たない（断られている間は本文を読まない R5 は保つ）
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）

## R17. `ch15_verifier_check_runs_off_the_async_worker` は、並走する試験が LOGIN_CHECK を握っていると変異 D2 を見逃す
- 成果物: crates/server/src/web_session_tests.rs（`ch15_verifier_check_runs_off_the_async_worker`）
- 根拠: 試験は 1 回 poll して Pending なら合格とする。印（`LOGIN_CHECK`）はプロセスで 1 つなので、並走する試験（R10〜R12 の取り消しの試験など）が握っていると印の待ちで Pending になり、async の worker の上で Argon2id を回す近道（R2 の変異 D2）でも通る。偽の赤にはならず、変異を捕まえるのが確率的になる（Minor）
- kind: technical
- 提案: 試験の中で先に印が取れることを確かめてから poll する（または `try_acquire` で空きを確かめる）か、Pending の理由が `spawn_blocking` であることを別の形（合図など）で見る
- 処置: fixed 1.2 —— 照合の本体を `check_with_permit`（印を持った後）に分け、試験は `hold_login_check_for_test` で印を先に取ってから `check_with_permit` を直に poll する。Pending の理由は blocking の側の照合だけになり、並走する試験が印を握っていても変異 D2 を捕まえる。変異 D2（`spawn_blocking` を外して async の worker で `accepts`）で FAILED、戻して PASS
- 判定: ADDRESSED（re-review 4 @f0a0150aecaa628b299e04fed301b777b130b196）
