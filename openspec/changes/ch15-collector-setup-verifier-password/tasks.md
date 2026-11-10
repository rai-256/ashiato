# CH15 実装タスク — 配った形のサーバで、収集アプリの設定が正しい合言葉を断る

読む順: `proposal.md` → このファイル → `design.md` → `openspec/specs/data-sensitivity/spec.md` の Requirement「画面は、ログインして画面の合言葉を入れ直したときだけ、収集アプリの設定の文字列を出す」→ `docs/changes/CH15.md` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: bug`）。実装の後に、実際の diff が予定のパスの中にあることと、名指しの Scenario の項目（1.1）が直す前に落ち、直した後に通った証跡を機械が見る。

## Global Constraints

- **触ってよいのは `lane.json` の予定のパスだけ**: `crates/server/src/web_session.rs`・`crates/server/src/web_session_tests.rs`・`crates/server/src/lib.rs`（試験の口が要るときだけ。今ある `App::with_login`・`web_session::password_verifier_for_test`・`login.failure_delay` で足りるので触らない。**1.1 の FAIL を記録するまで `lib.rs` と `web_session.rs` には触れない** —— 本番のコードに触れた木の FAIL は「直す前に落ちた」の証跡に数えられない）・`docs/changes/CH15.md`・`openspec/changes/ch15-collector-setup-verifier-password/**`。**`st55_tests.rs` / `st56_tests.rs`・`web/`・`docs/openapi.json` には触れない**（触れると通常の路へ昇格する）
- **正典の Scenario を変えない・`openspec/specs/` に触れない**。この change は spec の delta を持たない（`.openspec.yaml` の `skip_specs: true`）
- **照合はログインの口と同じ道に通す**（design D1）: `LOGIN_CHECK` の印・`spawn_blocking` の側での `WebLogin::accepts`・同じ側での `record_failure()`。`collector_setup_post` に `token_matches(…, &app.login.password)` を残さない
- **壊してはいけないもの**（`docs/changes/CH15.md`）: 口の応答の形と状態の番号（正しい合言葉 200 `{"token": …}`・違う / 空の合言葉 403 `{"error": "wrong_password"}`・印が無い 401・収集アプリの資格情報 403・JSON でない本文 415・数えを超えたら 429）・`Cache-Control: no-store`・失敗の遅れと、2 つの口で共有する失敗の数え・読み出しの記録の行と、記録とログに資格情報と入れた合言葉を出さないこと・ログインの口の振る舞い
- **試験の名前の接頭辞は `ch15_`**、置き場は `crates/server/src/web_session_tests.rs`。照らす形の App は `st55_tests.rs` の `st55_verifier_login_route_accepts_only_matching_password` と同じ組み方（`password_verifier_for_test` → `WebLogin::new_with_verifier` → `failure_delay = Duration::ZERO` → `App::with_login`）。合言葉と鍵は試験の中で作り、その合言葉でログインする（`App::with_web_login`・`App::web_password_for_test()`・`st56_tests.rs` の `login_cookie` の型は照らす形を組まない・空を返すので使わない）
- 試験の `// Scenario: <名前>` の印は正典の `#### Scenario:` と一字一句合わせる
- **長いコマンドは `scripts/quiet-run`**（cargo test）。本物の DB が要る（`.env` を読む。`docs/testing.md`）

## Task 1: 照らす形のサーバで、資格情報を返す口が正しい合言葉を受け付ける

- [ ] 1.1 Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る を試験で確かめる。直す前に verify-run で落ちることを記録する。
  **先に試験だけを書く**（`crates/server/src/web_session_tests.rs`。本番のコードはまだ触らない）:
  `ch15_verifier_collector_setup_returns_token`（`// Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る`。照らす形の App で `POST /session` にログイン → その印を付けて `POST /collector-setup` に正しい合言葉 → 200・`Cache-Control: no-store`・本文が `{"token": <App の収集アプリの資格情報>}`）/
  `ch15_verifier_collector_setup_wrong_password_is_forbidden`（`// Scenario: 合言葉が違えば出さない`。同じ App で 1 文字違いの合言葉と空の合言葉 → どちらも 403・本文 `{"error": "wrong_password"}`・資格情報を含まない・ログインの印は有効なまま）/
  `ch15_verifier_collector_setup_shares_login_failure_limit`（`// Scenario: 失敗が回数の制限を超えると、正しい合言葉でも答えない`。照らす形で、`st56_collector_setup_shares_login_failure_limit` と同じ手順 —— 資格情報の口で 9 回違う合言葉・ログインの口で 1 回失敗 → 正しい合言葉でも 429、空の合言葉も 429（design D1 の順））。
  書いたら、本番のコードが base のままの木で `scripts/verify-run 1.1` を走らせ、**FAIL**（`ch15_verifier_collector_setup_returns_token` が 403 で落ちる）を記録する。その後に design D1 のとおり `crates/server/src/web_session.rs` を直し（2 つの口が共に使う照合の関数を 1 つ置き、`session_post` と `collector_setup_post` がそれを使う）、もう一度 `scripts/verify-run 1.1` で **PASS** を記録する。
  直す前の FAIL のログに `ch15_verifier_collector_setup_returns_token` の失敗（403 と 200 の不一致）が出ていること（組み立ての失敗で落ちたのではないこと）を自分で確かめる（本数の下限は 1.2 が見る。1.1 に数えを前置きすると、試験が無い木の FAIL も「直す前に落ちた」と数えられる）。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server ch15_'` rc=0
- [ ] 1.2 今の振る舞いが残ることを確かめる（ログインの口・合言葉そのものの形の資格情報の口・照らす形のログイン）。`collector_setup_post` に `app.login.password` を比べる式が残っていないこと、照合が 1 か所（共通の関数）で `LOGIN_CHECK` の印の中にあることも見る。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server ch15_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0（`ch15_` が 3 本以上）/
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server st56_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7 && scripts/quiet-run ct -- cargo test -p ashiato-server st56_'` rc=0（`-- --list` で 7 本以上）/
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_ && scripts/quiet-run ct -- cargo test -p ashiato-server web_session'` rc=0 /
  `bash -c '! grep -n "login\.password\b" crates/server/src/web_session.rs'` rc=0 /
  `bash -c 'code=$(grep -vE "^\s*//" crates/server/src/web_session.rs); test "$(printf "%s\n" "$code" | grep -c "\.accepts(")" -eq 1 && test "$(printf "%s\n" "$code" | grep -c "LOGIN_CHECK")" -eq 2'` rc=0（コメントの行は数えない。`accepts` を呼ぶのは共通の関数の 1 か所、印は宣言と取る場所の 2 か所。async の側で `accepts` を呼ぶ近道を止める。base でも成り立つので赤から緑の確かめではない）/
  `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st56-collector-setup.spec.ts'` rc=0（合言葉そのものの形のサーバで画面の Scenario が通るまま。`docs/testing.md` §4.5）/
  `scripts/quiet-run clippy -- cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0 /
  `./tools/check-openapi.sh` rc=0（API の契約が変わっていない）

## 人間の確認待ち

- [ ] 9.1 配った形の確かめ（確認バッチ）: **この change を含む組み立ての配る道具（`tools/release/`）で上げた**本人の PC のサーバで（いま入っているのは不具合の起きた版。上げた版の commit を確認バッチの手順書で示す）、画面にログインし、設定の「収集アプリの設定」で同じ合言葉を入れて「表示」を押すと、設定の文字列とコピーのボタンが出る（「合言葉が違います」にならない）。違う合言葉では「合言葉が違います」と出ることも見る
  > 物理: realdata
