<!-- 処置の判定: re-review -->

# ST59 code_verify — 独立検証（2026-11-22）

検証した木: `feat/st59-change-password-from-web` @ `88d8156a9`（作業ツリーは clean）。DB は `ashiato2-st59-db-1`（healthy）。
壊す試験はすべて**作業ツリーの複製**（`.superpowers/sdd/tmp/a/mut/src`、別の `CARGO_TARGET_DIR`）で行い、1 件ずつ `cp` で元に戻して `cmp` で確かめた。作業ツリーのコードは触っていない。

## 申告: 14/14 `[x]`・検証コマンド一覧と実測

本文の検証コマンドを書かれたとおりに `bash -c` で順に走らせた（`scripts/verify-run` は使っていない。evidence.jsonl へは書いていない）。

| 項目 | 検証コマンド（要点） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `cargo test -p ashiato-server st59_swap_` + `--list` ≥ 4 | [x] | rc=0（5 本）。一致。ただし R2 |
| 1.2 | `web_session:16 st55_:47 st56_:7 ch15_:8` の走行と本数 | [x] | rc=0（63s）一致 |
| 1.3 | `cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records` + `--list` ≥ 1 / gradle `*St56SetupNotifierTest*` | [x] | rc=0 / rc=0（108s）一致 |
| 2.1 | `st59_settings_` + `--list` ≥ 5 / `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | [x] | rc=0 / rc=0 一致 |
| 3.1 | `st59_web_password_` + `--list` ≥ 10 | [x] | rc=0（12 本）。一致。ただし R3・R4・R5 |
| 4.1 | `st59_collector_token_` + `--list` ≥ 7 | [x] | rc=0（8 本）一致 |
| 4.2 | `st59_both_` + `--list` ≥ 8 / `./tools/check-openapi.sh` / `tools/check-log-private.sh` | [x] | rc=0 / rc=0 / rc=0。一致。ただし R5 |
| 5.1 | `tools/settings-server.sh --self-test` / `tools/check-secrets.sh` | [x] | rc=0 / rc=0 一致 |
| 6.1 | `npm run test:e2e -- st59-change-secrets.spec.ts` / `tsc && vitest && lint` | [x] | rc=0（157s）/ rc=0 一致 |
| 6.2 | `npm run test:e2e`（全体） | [x] | rc=0（566s）一致 |
| 7.1 | `tools/release/install-check.sh --self-test` / `--list` に ST59 の 3 本 | [x] | rc=0 / rc=0。一致。実地（CI の `install-windows`）は本文のとおり PR の CI に任せてあり、まだ走っていない（手 6） |
| 8.1 | `grep -q` 4 本 | [x] | rc=0 一致 |
| 8.2 | `grep -q` 3 本 | [x] | rc=0 一致 |
| 9.1 | fmt / clippy / windows-gnu clippy / `cargo test --workspace` / 検査 4 本 + `check_scenarios.py` | [x] | rc=0（588s。`cargo test --workspace` は quiet-run rc=0・552s）/ rc=0 一致 |

`python3 scripts/check_scenarios.py . st59-change-password-from-web` → `scenarios: OK` rc=0（担保あり 3187 / 3188。残りの 1 件は ST59 のものではない `[wait]`）。

## 手 1: 該当なし（確かめた範囲: 台本の固定の PHC 文字列）

`tools/settings-server.sh:39` の固定の PHC（`$argon2id$v=19$m=19456,t=2,p=1$gx8n…`）を、同じ行の試験用の合言葉 `st59-test-password-012345` から **OpenSSL 3.5.5 の `openssl kdf … ARGON2ID`**（Rust の `argon2` crate とは別の実装）で計算し直した → 32 バイトが一致（`match 32 32`）。
台本の値は、実装の出力を写したものではない。仕様どおりの Argon2id になっている。

## 手 2: ガードをわざと壊す

- `tools/release/install-check.sh` から `echo '== Scenario: 画面で変えた後も、インストーラで決め直せる'` の 1 行を消した複製 → `--self-test` rc=1（`NG: Scenario の印が実地の確かめに 1 つでない（0）`）。元の木は rc=0。守りは効いている
- `/web-password` の「呼び出し元が画面のログインか」を `caller == Caller::ApiToken` だけで断る形に変えた複製 → `st59_` は 38/38 で通るが、既存の `satellite_routes_tests::every_route_refuses_satellite` が FAILED（`satellite` で 50 passed / 1 failed、rc=101）。衛星の 403 は既存の試験が守っている
- 寿命の出し直しの守り（`web_session.rs:618` の `app.secrets_snapshot().tag() == secrets.tag()`）を消した複製 → `st59_swap_refresh_does_not_set_cookie_after_replacement` が FAILED（rc=101）。守りは効いている
- `tools/check-log-private.sh` は、合言葉をログに出す変異でも rc=0（R5）

## 手 3: Scenario と test の突き合わせ

印は全部付いている（`check_scenarios.py` OK）。印の先が Scenario の主張を観測しているかを、変異で確かめた結果が R2〜R5。
ほかに読んだもので指摘にしないもの: `本文が JSON でない求めでは、何も変わらない` の印の先（`st59_both_non_json_is_rejected`）は `/collector-token` しか送らず、設定のファイルを比べない。`/web-password` の 415 は印の無い `st59_web_password_handler_rejects_non_json` が見ており、415 で返る経路は書き戻しに届かないので、変異で壊せる振る舞いが無かった。

## 手 4: 該当なし（確かめた範囲: 本人の答え Q1〜Q3）

本人の答えは Q1〜Q3（どれも推奨のまま）。数値は持たない。
- Q1（変えた画面だけログインしたまま）: 変えた応答の `Set-Cookie` を返さない変異 → `st59_web_password_handler_revokes_other_cookies_and_returns_new_one` が FAILED（rc=101）。固定されている
- Q2（2 つの節）: e2e `設定に 2 つの節が並び…` が 2 つの見出しと 1 行を名指しで見る（6.1 rc=0）
- Q3（開発の形は 409 と文言）: `st59_web_password_handler_rejects_development_mode` が `{"error":"not_changeable"}`、e2e が文言を `toHaveText` で完全一致で見る（6.1 rc=0）

聞かない既定 C5 の「16 文字」は、本人の答えではないので R3 に書いた。

## 手 5: tasks の `[x]` と実体

上の表のとおり、14 項目の検証コマンドはどれも実在し、rc=0 だった。`cargo test <名前>` 型はどれも `--list` で本数を見ている。
本文に名前の挙がった試験は全部ある。ただし、1.1 の本文の試験（`照合が通った後・印の行を書く前に組を差し替え`）と 3.1 の本文の試験（`書き直した server.json から組み立て直した App`）は、名前のとおりのものがあっても、本文の主張を観測していない（R2・R4）。

## 手 6: 隙間

- R1: 照合の前に取った組で照合する 2 つの口が、合言葉を変えた後に前の合言葉を通す（実測）
- IMP-04（インストーラの `password` との競り合い）は `python3 scripts/imp_ledger.py risks .` の 4 行目に `ST59 IMP-04: 判断し直す（責任者 本人・時期 ST58 の上流の合意まで）` として載っている。機械が追っているので指摘しない
- 衛星の資格情報が残る件は `docs/handoff/ST38.md:84` に節がある。ST60 への申し送りは 8.2 の grep が rc=0
- 変えた収集アプリの値は応答の本文にしか出ない。ただし応答を受け取れなくても、`/collector-setup` で出し直せる（ST56）ので失われない
- premises 5 行目（画面で変えた後も、インストーラの `password` で決め直せる）は確かさ 2 のまま。Windows の実物ではまだ一度も走っていない:
  `gh pr list --head feat/st59-change-password-from-web --state all` → `[]`、`gh run list --branch feat/st59-change-password-from-web` → `[]`。evidence.jsonl の 7.1 の 4 件は、どれも手元の `--self-test` と `--list` だった。
  ただし tasks 7.1 は、もともと PR の CI に任せる予定になっている。CI での担保の経路は次のとおり実在するので、指摘にはしない:
  - この変更の差分は `.github/workflows/ci.yml:59` の正規表現に当たる（`git diff --name-only origin/main HEAD | grep -qE …` → `windows=true`）ので、`install-windows` が走る
  - `scripts/merge_gate.sh:102` の `ci_wait.py` は、全 workflow の最新の run が `success` か `skipped` のときだけ通す
  - ST59 の 3 本の印を 1 本消すと `--self-test` が rc=1 になる（手 2）
  CI が赤なら D5 の反転条件に入る

## 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st59-change-password-from-web` → `[]`）。`correction-review.md` は作っていない。

---

## R1. 合言葉を差し替えた後に照合されたログインと「収集アプリの設定」が、前の合言葉で通る（差し替えの前から照合の印を待っていたもの。「収集アプリの設定」は今も効く値を返す）
- 成果物: `crates/server/src/web_session.rs`（`session_post_checked` の `let secrets = app.secrets_snapshot();` が `check_given_password_with_secrets` の中の `login_check_permit().await` より前。`collector_setup_post` は middleware が求めの初めに取った `REQUEST_SECRETS` で照合する）/ design D1「1 つの求めは、組を最初に 1 回だけ取る」
- 根拠: 複製に一時の試験を 2 本足して実行した（`flavor = "multi_thread"`。実行後は消し、`cmp` で元と同じを確認）。
  (a) 順を決めた回: 試験が `hold_login_check_for_test()` で照合の印を持つ → 前の合言葉のログインと、もう 1 つのログインの印での `POST /collector-setup`（前の合言葉）を送り、`check_waiting_for_test()` で 2 つとも印を待っているのを確かめる → **印を放す前に** `replace_secrets_for_test("old-api-token-012345", "new-web-password-012345")` で差し替える → 印を放す。これで照合は必ず差し替えの後になる。結果:
  `other cookie now reads 401`（差し替えの時点で、もう 1 つの印は締め出されている）/ `login(old pw) checked after swap: status=204` / `collector-setup(other cookie, old pw) checked after swap: status=200 live_token=true`。
  (b) 本物の口の回: 印を持ったまま本物の `POST /web-password` を先に並べ（300ms 待つ）、続けて同じ 2 つを並べてから印を放す。結果は同じだった:
  `change 204` / `login 204`（その印での読み出しは 401）/ `collector-setup 200 live_token=true`。この回の「変える口が先に印を取った」は、印の待ちの順からの推定。
  読み出しの記録には `("/session","login_ok",204)` と `("/collector-setup","ok",200)` が残る。その後に同じ求めを送ると、`login 401`・`collector-setup 401` になる。
  spec「変える処理と、ほかの口の合言葉の照合を 1 つずつ通し、変える前の合言葉で照合が通ったログインに、変えた後も使える印を渡さない」の 1 つずつは守られているが、照合は**変える前の組**で行われる。その結果、締め出したはずのログインの印と前の合言葉で、収集アプリの値（変えた後も有効）を受け取れる。また、前の合言葉のログインに 204 を返し、読み出しの記録に `login_ok` と残す（「前の合言葉のログインは断られ」と食い違う）
- kind: technical
- 提案: 照合に使う組を、`LOGIN_CHECK` の印を取った**後**に取り直す（`check_given_password_with_secrets` の中で、印の後に `app.secrets_snapshot()`。`/collector-setup` は middleware の組の `tag` と印の後の組の `tag` が違えば 401）。上の並べ方を `st59_` の試験にする
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R2. Scenario「変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない」の担保が無い（印の先は別の主張を見ている。印の無い試験も含め、サーバの単体は壊した実装で全部通る）
- 成果物: `crates/server/src/st59_tests.rs:137-158`（印の先 `st59_web_password_rechecks_secrets_after_serialization_permit` は、変える口の照合の間に共有の値を差し替えて 400 `same_as_api_token` を見る。ログインの印は見ていない）/ `:1256-1282`（印の無い `st59_swap_login_cookie_uses_the_old_tag_and_is_revoked_after_swap`）/ tasks 1.1・3.1
- 根拠: ログインの口の印の行を、照合した組の `tag` ではなく書く時点の組の `tag` で書く変異（`web_session.rs:1408` の `.bind(secrets.tag())` → `.bind(app.secrets_snapshot().tag().to_vec())`）→ `cargo test -p ashiato-server st59_` が **38 passed / 0 failed、rc=0**。印の無い `st59_swap_login_cookie_…` だけを 5 回走らせても 5 回とも `ok`。
  この変異と、R3・R5 の変異を同時に入れた複製で `cargo test -p ashiato-server --lib` → **2035 passed / 0 failed / 3 ignored、rc=0**（サーバの単体のどれも落とさない）。
  `#[tokio::test]` は 1 本の thread で動くので、`check_succeeded` の通知から印の行の `bind` までの間に試験の差し替えが割り込めない。ログインは照合の後に印を放してから印の行を書く（`check_given_password_with_secrets` の中の印は関数を出ると放す）ので、本番では変える口がこの間に割り込める
- kind: technical
- 提案: `flavor = "multi_thread"` にし、照合の後・印の行の INSERT の前で止める差し込み口（`#[cfg(test)]`）を足して、そこで本物の `POST /web-password` を通す試験に、この Scenario の印を移す。上の変異で落ちることを確かめる
- 処置: fixed 1.1
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: st59_tests.rs の差分は 724 行付近と 826 行付近の 2 か所だけ。ログインの印を見る試験（multi_thread・差し込み口）が無い
- 判定: NOT ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）: st59_tests.rs に multi_thread・差し込み口を使うログインの印の試験が足されていない。st59_swap_login_cookie_uses_the_old_tag_and_is_revoked_after_swap（st59_tests.rs:1350）は印の無い単一 thread のまま。tag を書く時点の組にする変異を落とす試験が無い
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R3. Scenario「新しい合言葉が検査に当たれば、何も変わらない」の理由の文字列と 15 文字の境界は、サーバの単体のどれも見ていない
- 成果物: `crates/server/src/st59_tests.rs:950-983`（`st59_web_password_handler_applies_all_five_validation_rules` は状態の 400 だけを見て本文を読まない。短すぎの入力は `"short"`（5 文字））/ `web_session.rs` の `validate_new_web_password` / spec「どれも 400 で、本文の理由はそれぞれ `too_short`・`placeholder`・…」「15 文字の合言葉」
- 根拠: `validate_new_web_password` の 1 つ目の検査を `if new_password.chars().count() < 6 { return Err("mismatch");` に変える変異（下限 16 → 6、理由 `too_short` → `mismatch`）→ `cargo test -p ashiato-server st59_` が **38 passed、rc=0**（R2・R5 の変異と同時に入れた `--lib` 全体も 2035 passed、rc=0）。e2e にも理由ごとの文言を見る試験は無い（`web/e2e/st59-change-secrets.spec.ts` は 400 を送らない）。`grep -c 'too_short\|placeholder\|same_as_session_key\|"mismatch"' crates/server/src/st59_tests.rs` → `0`（ST59 の試験に理由の文字列は 1 件も無い）。
  画面は理由ごとに別の文言（D11）を出すので、理由を取り違えると、本人に違う直し方を示す
- kind: technical
- 提案: 5 つの入力（短すぎは **15 文字**ちょうど）ごとに本文 `{"error": "<理由>"}` を `assert_eq!` で固定し、16 文字は 400 にならないことも 1 件見る
- 処置: fixed 3.1
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: 理由の文字列（too_short など）と 15 文字の境界を固定する試験が差分に無い
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R4. Scenario「変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない」の試験は、設定のファイルを読まずに新しい合言葉を直接渡して組み立てている
- 成果物: `crates/server/src/st59_tests.rs:1187-1221`（`let rebuilt = app.with_web_login("new-web-password-012345", 0);`）/ tasks 3.1「書き直した `server.json` から組み立て直した `App` で」
- 根拠: 変える口の書き戻しを消す変異（`if crate::service::replace_setting("WEB_PASSWORD_VERIFIER", &verifier).is_err() {` → `if false {`）→ `st59_web_password_` と `st59_both_` を走らせて **この試験は `ok`**（落ちたのは `st59_web_password_handler_updates_only_the_verifier_key` と `st59_both_write_failure_returns_not_saved` の 2 本だけ）。
  収集アプリの値の側（`st59_collector_token_uses_new_token_after_rebuilding_app`）は `load_settings` で読み直しているのに、合言葉の側は読んでいない。
  この Scenario の起動し直しを本物で見るのは `install-check.sh` だけで、それは PR の CI の `install-windows` まで走らない（手 6）
- kind: technical
- 提案: `service::load_settings` で書き直した `server.json` を読み、その `WEB_PASSWORD_VERIFIER` で `WebLogin::new_with_verifier` を組んだ `App` で、前の合言葉は 401・新しい合言葉は 204 を見る（`st59_collector_token_uses_new_token_after_rebuilding_app` と同じ形）
- 処置: fixed 3.1
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: st59_tests.rs の起動し直しの試験は未変更で、with_web_login で直に渡している。load_settings で server.json を読み直していない
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R5. Scenario「…値は、記録にもログにも応答の見出しにも残らない」の「ログ」は、サーバの単体も `check-log-private.sh` も見ていない
- 成果物: `crates/server/src/st59_tests.rs:738-805`（`st59_both_change_values_are_not_logged` は読み出しの記録の行と応答の見出しだけを見る。`:1223-1243` も読み出しの記録だけ）/ `tools/check-log-private.sh`
- 根拠: `web_password_post_inner` の照合の前に `tracing::warn!(new = %request.new_password, current = %request.password, "st59 mutation");` を足す変異 → `cargo test -p ashiato-server st59_both_change_values_are_not_logged` **ok、rc=0** / `tools/check-log-private.sh` **rc=0（`check-log-private: OK`）** / R2・R3 の変異と同時に入れた `--lib` 全体も 2035 passed、rc=0。
  tasks 3.1 の本文「入れた合言葉と照らす値がサーバのログと読み出しの記録に無い」も、ログの側は確かめていない
- kind: technical
- 提案: 試験の中で `tracing` の出力を集め（既存の試験にログを捕まえる部品があればそれを使う）、2 つの口を成功・失敗で通した後に、今の合言葉・新しい合言葉・新しい照らす値・新旧の収集アプリの値が無いことを見る
- 処置: fixed 4.2
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: tracing の出力を集めて値が無いことを見る試験が差分に無い
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## security review（56fd1c5..8dddaff）

見た経路:
- `POST /api/web-password` → `refresh_cookie`（`decide_with_secrets`）→ `access_log::middleware` は素通し（`access_log.rs:166-169` に今回足した）→ `web_password_post_inner` の `caller != Caller::WebSession` で 403（`web_session.rs:727`）→ 今の合言葉の照合（`LOGIN_CHECK` の印と失敗の数え上げはログインと共有）→ `validate_new_web_password` → Argon2id の照らす値 → `service::replace_setting("WEB_PASSWORD_VERIFIER")` → `replace_secrets`（2）
- `POST /api/collector-token` → 同じ素通し → `collector_token_post` の `caller != Caller::WebSession`（`web_session.rs:1047`）→ 今の合言葉の照合 → `OsRng` の 32 バイト → `replace_setting("API_TOKEN")` → 本文に新しい値（2）
- 認証なし → 401、収集アプリの値 → 403、衛星の資格情報 → 403（`/collector-token`）を、`cargo test -p ashiato-server --lib rejects_non_web_callers` で確かめた → 2 passed（4）
- `collector_scope::credential_exempt` に足した `/collector-token` は、`bin/openapi.rs:36` だけが読む（契約の 403 の有無）。実行時の判定には使われない（`grep -rn credential_exempt crates/server/src` で確かめた）（3）
- 設定の書き戻し: `replace_setting_at` は鍵を `WEB_PASSWORD_VERIFIER` / `API_TOKEN` に限る。パスは環境の `ASHIATO_SETTINGS_PATH` だけから取り、求めの値は入らない。一時ファイルの名前は `server.json.writing` で決まっている（`service.rs:268-291`）（3）
- Windows の ACL: `server.json` に明示の ACL は無く、状態の置き場の根から継承している（`commands.rs:468-483`。サーバのサービスに M、本人に RX）。同じ `settings\` に作る `server.json.writing` も同じ ACL を継承するので、名前を替えた後も読める主体は増えない。`owner.json` は明示の ACL で、書き戻しは触れない（3）
- 秘密がログ・記録に出る経路: 2 つのハンドラに `tracing::` は無い（`web_session.rs:640-1260` を grep した）。`SettingsWriteError` の `Display` は理由の名前だけを出す。読み出しの記録は口の名前と結果だけ。`SharedSecrets` の `#[derive(Debug)]` を `{:?}` で出す箇所は無い（前の `WebLogin` にも、同じ形で合言葉が入っていた）（3）
- 画面: 2 つの部品は送った後に入力欄を空にする。新しい値は `readOnly` の入力欄とクリップボードにだけ出す。e2e は値を console に出さない（2）
- `tools/settings-server.sh` は試験用の固定値（`st59-fake-…`）で、`umask 077` で書く。待ち受けは `127.0.0.1`（2）

安全の決め手: 2 つの新しい口は `access_log::middleware` の「収集アプリの値は許可の一覧の口だけ・衛星は全部 403」（`access_log.rs:182-186`）を通らない。呼び出し元を断るのは、ハンドラの `caller != Caller::WebSession`（`web_session.rs:727` / `:1047`）だけになる —— 確かさ 4（`/collector-token` の側）。
台本: `web_session.rs` の 3 か所の `if caller != Caller::WebSession {` を `if false && …` に変えて `cargo test -p ashiato-server --lib rejects_non_web_callers` → `st59_collector_token_rejects_non_web_callers` が `left: 200 / right: 403`（st59_tests.rs:297）で落ちた。戻した後は `cmp` と `git diff --quiet` で元と同じことを確かめた。
`/web-password` の側は、同じ変異で試験が**通った**（R6）。決め手は `file:line` で示せるだけで、試験としては担保がない（確かさ 2）

## R6. `/web-password` の「収集アプリの値・衛星では変えられない」の試験は、呼び出し元の判定を外しても通る（新しい口は middleware の収集アプリの値の判定を外したので、ここが唯一の判定）
- 成果物: `crates/server/src/st59_tests.rs:987-1013`（`st59_web_password_handler_rejects_non_web_callers`）/ `crates/server/src/access_log.rs:166-169` / `crates/server/src/web_session.rs:727`
- 根拠: ハンドラの `if caller != Caller::WebSession {` を `if false && caller != Caller::WebSession {` に変える変異 → `cargo test -p ashiato-server --lib rejects_non_web_callers` で、`st59_web_password_handler_rejects_non_web_callers` は **ok**（`/collector-token` の側は 200≠403 で落ちた）。
  試験の組み立ては `App::for_test(…, "api-token-0123456789")` で、合言葉は `test-web-password-0123456789`（`lib.rs:690`）。それなのに `Bearer` の求めに `"old-web-password-012345"` を載せている。そのため、判定を外しても照合で `403 wrong_password` になり、期待の 403 と区別できない。衛星の資格情報の場合はこの試験に無い（Scenario の名前は「衛星の資格情報では、合言葉を変えられない」を含む）。
  `access_log.rs:166-169` が `/web-password` を middleware の判定（`:182` の `collector_allows`・`:186` の衛星は 403）から外したので、ハンドラの 1 行が消えると、携帯に入っている収集アプリの値（落とした端末に残るもの）で画面の合言葉を変えられる。ただし今の合言葉は要る。今の実装にはこの 1 行があるので、穴ではない。この 1 行を守る試験が無い（確かさ 4）
- kind: technical
- 提案: `Bearer <収集アプリの値>` と衛星の資格情報の求めに、**正しい今の合言葉**を載せる。状態 403 で、本文が `wrong_password` でないこと、`server.json` が変わらないことを見る。上の変異で落ちることを確かめる
- 処置: fixed 3.1

範囲外: `access_log::middleware` の素通しの一覧（`access_log.rs:166`）と `collector_scope::credential_exempt` は、別々の一覧を手で揃えている。今回も 2 か所に同じ口を足した（既存の形。R 番号は振らない）
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: st59_web_password_handler_rejects_non_web_callers は未変更。正しい今の合言葉を載せた求めも衛星の資格情報の求めも足していない
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## /code-review（56fd1c5..12530a6）

見た: Claude Code の組み込みの `/code-review`（7 件）。写しは scripts/code_review_record.py

## R7. The install check fails at the collector-token step with a 401. The step reuses `$changed_cookie` after the earlier `/web-password` POST, but that change gives the session a new secret tag, so the old cookie is no longer valid. The server sends the replacement cookie in `Set-Cookie`, and the curl call never saves it. Fix: save the new `ashiato_session` from the `/web-password` response (`-c`/`-D`), or log in again with `$PW3` before POSTing `/collector-token`.
- 成果物: tools/release/install-check.sh:1150
- 根拠: Windows install-check: the `/web-password` call returns 204 and replaces `secrets` with a new tag. The `/collector-token` call then sends the old cookie, `session_valid_with_secrets` finds no row with the new tag, and the server answers 401. The script stops at `die '収集アプリの値の変更に答えない'`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 7.1
- 判定: NOT ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）: install-check.sh 新 1138 行。#HttpOnly_ 行を startswith('#') で捨てる。単引用符内の "\\t" は 2 文字の \t でタブにならず IndexError。changed_cookie が空になり die で止まる。サーバは HttpOnly を付ける（web_session.rs:566）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）

## R8. Login and `/collector-setup` can still accept the old secrets after a change has finished. Both take their secrets snapshot before they wait for the `LOGIN_CHECK` permit: `session_post_checked` reads `app.secrets_snapshot()` early, and `collector_setup_post` uses the snapshot taken in `refresh_cookie`. A request queued behind a `/web-password` or `/collector-token` change is therefore checked against the old password, and its answer is built from the old values. Fix: take the snapshot after the permit is acquired, inside `check_given_password_with_secrets` or right after it, and use that snapshot for the tag and the token.
- 成果物: crates/server/src/web_session.rs:1372
- 根拠: Request A is changing the password and holds the permit. Request B logs in with the old password, takes its snapshot, and waits. A finishes and swaps the secrets. B is then accepted with the old password and gets a 204 and a cookie carrying the old tag, which every later request rejects with 401. The same thing happens with `/collector-setup` during a token change: the phone receives the old API token, which is already invalid.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R9. When saving fails, the access log still records a successful change. `web_password_post_inner` writes the `ok` row and inserts a new `core.web_session` row before it calls `replace_setting`. If the save then fails, the log holds both `ok`/204 and a 500 row, and the session row stays behind with a tag that never becomes valid. `collector_token_post_checked` (around line 1146) writes its `ok` row before `replace_setting` in the same way. Fix: write the `ok` row, insert the session, and set the cookie only after `replace_setting` succeeds, or delete the session row when the save fails.
- 成果物: crates/server/src/web_session.rs:837
- 根拠: `server.json.writing` cannot be created (the e2e test makes it a directory). The response is 500 `not_saved`, but the access log contains a `/web-password` `ok` 204 entry, so an audit reports a password change that never happened.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D9
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R10. Argon2id hashing and verification now run on the async worker. `password_verifier` hashes inline in `web_password_post_inner`, and in `collector_token_post_checked`, `generate_collector_token` runs `check_web_password_verifier` and `check_web_session_key_verifier` (two Argon2 verifies) inline, plus a hash in the Plain case. `replace_setting` also does synchronous file I/O and `sync_all` there. All of this happens while the global `LOGIN_CHECK` permit is held. This breaks the rule from review R23 that Argon2 work runs on the blocking side. Fix: wrap the hashing, the token generation and `replace_setting` in `spawn_blocking`.
- 成果物: crates/server/src/web_session.rs:818
- 根拠: A password or token change keeps a tokio worker busy for tens of milliseconds or more, and the fsync adds more. Every other request on that worker stalls, and every login waits behind the permit.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D4
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R11. Rewriting `server.json` can loosen its file permissions and leaves a secrets-bearing temp file behind on failure. `replace_setting_at` opens `server.json.writing` with the default mode (0666 minus the process umask, so 0644) and renames it over `server.json`. The original file was created 0600 (`umask 077` in `settings-server.sh`). If `write_all`, `sync_all` or `rename` fails, the half-written file with every secret stays on disk. Fix: on Unix, copy the original file's permissions to the temp file (or create it 0600), and remove the temp file on every error path.
- 成果物: crates/server/src/service.rs:298
- 根拠: On Linux the service-mode server (`tools/settings-server.sh up`) changes the token. `server.json` goes from 0600 to 0644, so any local user can read `API_TOKEN`, `WEB_SESSION_KEY` and `DATABASE_URL`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）

## R12. The permit is no longer released only after a failure has been counted. Before this change the permit was moved into `spawn_blocking` and dropped after `limiter.record_failure()` (review R78). `check_with_permit_using_secrets` now only borrows it (`let _ = permit`), and the caller's future owns it. If that future is dropped while it awaits `spawn_blocking`, the permit is released before the failure is counted. `run_to_completion` hides this today, but the guarantee is gone, and `check_with_permit` called directly (as in the tests) shows it. Fix: pass an owned permit into the blocking closure again, for example `Arc` or the owned guard.
- 成果物: crates/server/src/web_session.rs:1305
- 根拠: A caller that is not wrapped in `run_to_completion` is cancelled during the Argon2 verify. The next check takes the permit and reads the limiter before the previous failure is recorded, so it gets past the 10-failure limit.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: NOT ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）: web_session.rs:1447-1452 runs the blocking check without the permit or limiter, and record_failure() runs afterwards in the caller's future. If that future is dropped during the await, the permit is released before the failure is counted, or the failure is never counted. The R19 change reintroduced the original defect.
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R13. While login is throttled, the new endpoints still queue on the global permit before refusing. `check_given_password_with_secrets` checks `limiter.throttled()` before it waits for the permit and before it reads the body (reviews R4 and R5). `web_password_post_inner` and `collector_token_post_checked` call `login_check_permit().await` directly, so they skip that early check. Fix: check `app.login.limiter.throttled()` before acquiring the permit, as `check_given_password_with_secrets` does.
- 成果物: crates/server/src/web_session.rs:773
- 根拠: During throttling, many `/web-password` or `/collector-token` requests line up behind the permit and delay ordinary login answers, instead of getting an immediate 429.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D4
- 判定: ADDRESSED（re-review 1 @30859ae9541d696554aba2fee5e380d5a77e59e8）
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## settle_review（re-review 1 @30859ae）: 処置の直しが新しく壊したもの

## R14. 変える口が許可（LOGIN_CHECK）を照合に所有ごと渡すので、照合の直後に放される。その後のハッシュ・保存・replace_secrets は許可なしで走る。この間に別のログインが旧い組で通り、旧い tag の印を受け取る（R1・R2 の競合の再発）。変える口どうしが同時に旧い値で通ることもある。許可は replace_secrets の後まで持つこと（collector_token_post_checked の 1150 行付近も同じ）。
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:155（re-review 1 の New Breakage。Important）
- kind: technical
- 処置: fixed D1
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R15. web_password の core.web_session INSERT が replace_setting の後なので、INSERT の失敗で server.json だけ新しい値に変わり、メモリの組は旧いまま残る。ok 行の失敗には巻き戻しがあるが、INSERT の失敗には無い。
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:236（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed D4
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R16. settings_before の同期 std::fs::read が async 側にある。巻き戻しの std::fs::write は非原子的。
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:208（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed D4
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R17. access-log 失敗の試験を、server.json のバイト比較から JSON 比較へ緩めた。理由は報告に無い。
- 成果物: crates/server/src/st59_tests.rs
- 根拠: crates/server/src/st59_tests.rs:826（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 4.2
- 判定: ADDRESSED（re-review 2 @8e061402877bccf1c689740a37d8471631ab56c3）
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## settle_review（re-review 2 @8e06140）: 処置の直しが新しく壊したもの

## R18. 単引用符内の rstrip("\\n") は Python では文字 \ と n を末尾から削る指定になり、cookie の値が n で終わると 1 文字削れる。今のトークン（43 文字の URL-safe base64）の最後の文字は n にならないので今は壊れないが、潜在する。
- 成果物: tools/release/install-check.sh
- 根拠: tools/release/install-check.sh:1138（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 7.1
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）

## R19. unwrap_or_else(record_failure) を expect("照合の blocking task は panic しない") に変えた。blocking task が cancel されたとき（ランタイム終了など）、失敗に数えずに panic する。
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:check_with_permit_using_secrets（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed D1
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R20. 巻き戻しの restore_setting の失敗を let _ で捨てている。巻き戻しが失敗すると、server.json だけ新しい値のまま無言で残る。
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:web_password_post_inner（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed D4
- 判定: ADDRESSED（re-review 3 @fa28ee998fa5fd727c682ff8a9b666dc67bb1978）
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## settle_review（re-review 3 @fa28ee9）: 処置の直しが新しく壊したもの

## R21. The R19 change took the permit and limiter out of the blocking closure. If the check future is dropped during the await, the permit is released before the failure is counted, and the failure may not be counted at all. This is the same defect as R12 and the R78 guarantee, and nothing tests it. Fix: move the permit and limiter into the closure again, and treat a JoinError as a counted failure inside the closure's owner.
- 成果物: crates/server/src/web_session.rs
- 根拠: crates/server/src/web_session.rs:1447（re-review 3 の New Breakage。Important）
- kind: technical
- 処置: fixed D1
- 判定: ADDRESSED（re-review 4 @ee677eb666b27ffaff72da4fdae6281d3ead1981）
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## final review（56fd1c5..2b27dc1）

whole-branch の final reviewer（SDD の `code-reviewer.md`。package は `.superpowers/sdd/tasks/review-56fd1c5a8..2b27dc18b.diff`）の指摘。R1〜R21 と重なるものは除いた。Task ごとの ⚠️ の確かめ: `replace_setting` の原子性・失敗の数えの共有（4 つの口が同じ `app.login.limiter`）・ポートの一致・PHC の値・e2e の PASS は問題なし。Windows の rename と新しい `server.json` の ACL（T2 ⚠️・T7 ⚠️）は未検証のままで、PR の CI の `install-windows` の緑で確かめる。

## R22. HEAD で `cargo clippy -p ashiato-server --all-targets -- -D warnings` が落ちる（rc=101。settle の直しで呼び手の無くなった関数が 2 つ残る）
- 成果物: `crates/server/src/lib.rs`（`:1000` の `request_secrets`、`:783` の `replace_secrets` の要らなくなった `#[allow(dead_code)]`）/ `crates/server/src/web_session.rs`（`:1414` の `check_given_password_with_secrets`）
- 根拠: final reviewer が HEAD で走らせて `error: function ... is never used`（lib と lib test の両方。`ci.yml` の rust job は `-D warnings`）。`grep -rn 'request_secrets\b\|check_given_password_with_secrets' crates/server/src` は定義の 2 行だけを返す。evidence.jsonl で 9.1 の最後の PASS は `1303a7c9d` で、その後の settle の 4 commit の後に 9.1 の全体を取った証跡が無い
- kind: technical
- 提案: 2 つの関数を消す（`REQUEST_SECRETS` の scope も読み手が無ければ外す）。HEAD で 9.1 の全体（fmt・clippy の linux と windows-gnu・`cargo test --workspace`・`check-openapi.sh` ほか）を `scripts/verify-run 9.1` で取り直す
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R23. R9 の処置で、変える口の読み出しの記録の順が spec と design の定めと逆になった。Scenario の印の先の試験も、印と逆の主張を見ている
- 成果物: `crates/server/src/web_session.rs`（`:885-906` の `/web-password`、`:1249-1270` の `/collector-token`）/ `crates/server/src/st59_tests.rs`（`:677-772`）
- 根拠: `openspec/changes/st59-change-password-from-web/specs/data-sensitivity/spec.md:67`「変えたものの行は値を書き戻す前に…書き、書けなければ何も変えずに 500 で断る。行を書いた後に値を書き戻せなかったときは…2 行とし…打ち消しの行を足す」、同 `:144-148` の Scenario、`design.md:40-41`。実装は書き戻しの**後**に `ok` を書き、書けなければファイルを巻き戻す。試験は `let expected = if outcome == "ok" { old } else { old + 1 }` で `ok` の行が増えないことを確かめているのに、印は「書き戻しに落ちた求めは、変えた行と打ち消しの行の 2 行で残る」のまま（`check_scenarios.py` は素通りする）。今の順では、`ok` を書けず巻き戻しも落ちると（R20 はエラーを出すだけ）、`ok` の行が無いまま起動し直しで変更が効く。spec の順は「監査の行が先に必ずある」（深掘り C5、loss: uncaptured）を守るためのもの
- kind: premise
- loss: uncaptured —— 変更が効いたのに読み出しの記録に `ok` の行が無い期間ができうる
- 提案: spec の順（`ok` → 書き戻し → 失敗なら打ち消しの `forbidden`/500 の行）に戻し、試験を印の主張（2 行で残る）に合わせる。R9 の順を採るなら、spec の Requirement と Scenario を正規の経路で直してから印を付け直す
- 処置: escalated —— 答え済み（deep.md 第 2 回 Q4。`deep-answers-3.txt`）: 本人が「変えた」（`ok`）の行を先に書き、書き戻しに落ちたら打ち消しの `forbidden`/500 の行を足す順（仕様の本文・design D9 どおり。推奨の側）を選んだ。コードと試験は 04cfe42dd でこの順に戻してあり（`/web-password`・`/collector-token` とも 印の行 → `ok` の行 → 書き戻し → 差し替え。試験 `st59_both_write_failure_returns_not_saved` は `ok` の行も `old + 1` と打ち消しが後に並ぶことを見る）、spec・design の直しは無い。HEAD で `cargo test -p ashiato-server st59_both_` 8 passed

## R24. 申し送りの「値を変えた後」の案内のうち、「全部の収集アプリに貼り直す」の 1 文が画面に無い
- 成果物: `web/src/messages.ts`（`secretChange.setupLead`）/ `web/src/CollectorTokenChangePanel.tsx`（`:36`）/ `web/e2e/st59-change-secrets.spec.ts`（collector-token の試験）
- 根拠: 申し送り「st56-collector-connection-setup R2（review/spec.md）・D8 — 合言葉の入れ直しの口と、値を変えた後の知らせは ST56 が作る（2026-11-04、ST56 の上流 spec。第 1 回の推奨で先に書いた版）」（`docs/handoff/ST59.md:8`「変える画面は、変えた後に…全部の収集アプリに貼り直す、と案内する」）と `specs/**/spec.md:273` の案内「全部の収集アプリ（携帯と PC）にこの文字列を貼り直してください。携帯に入れる文字列は、携帯から開いた画面で『収集アプリの設定』から出してください」。`grep -rn 貼り直 web/src` は 0 件で、実装は 2 文目だけを持つ。e2e は 2 文目の部分一致しか見ないので通る。申し送りの前半（照合の関数・失敗の遅れ・回数の制限をログインとそろえる）は守られている（`web_session.rs:820-821`・`:1189-1190`・`:1410`、`app.login.limiter` を共有）
- kind: technical
- 提案: 1 文目を足し、e2e で案内の全文を見る
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）

## R25. ST59 の試験と ST55 の試験が、別の鍵で同じ環境変数（`ASHIATO_SETTINGS_PATH`・`ASHIATO_SERVICE_MODE`）を書き換える。並列に走ると時々落ちる
- 成果物: `crates/server/src/st59_tests.rs`（`:67` の `ST59_SETTINGS_LOCK`、手で戻している `:593-620`・`:899-920`・`:1229-1251`）/ `crates/server/src/st55_tests.rs`（`:14` の `ENV_LOCK`、`:607-619`）
- 根拠: handler は求めのたびにこの 2 つをプロセスの環境から読む。別の鍵なので、並列に走ると ST59 の handler が 409 を返すか、ST55 の一時ファイルに書く。Task 2 F1 と同じ種類で、Task 3 F12・Task 4 F14・Task 9 F3（手で環境を戻すので assert が落ちると戻らない）の残り
- kind: technical
- 提案: 鍵を 1 つにする（`ENV_LOCK` を `pub(crate)` にして共有する）。手で戻している所を drop で戻す guard にそろえる
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R26. 今の合言葉が違ったとき、変える 2 つの口は照合の印（`LOGIN_CHECK`）を持ったまま失敗の遅れを待つ。その間ほかの全部のログインが止まる（Minor）
- 成果物: `crates/server/src/web_session.rs`（`:833-843`、`:1202-1218`）
- 根拠: ログインの口は印を放してから待つ（`:1523`→`:1548`）。遅れの長さは同じだが、変える口で 1 回間違えるたびに全部のログインが 1 秒止まる（回数の制限があるので上限はある）
- kind: technical
- 提案: 印を放してから `failure_delay` を待つ
- 処置: fixed D1
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R27. 変える口の 500 の一部で、原因がログにも読み出しの記録にも残らない（Minor）
- 成果物: `crates/server/src/web_session.rs`（`:890-906`・`:1253-1269` の `not_saved` の `_ =>`、`:920` の印の行の INSERT の失敗、`:864-883` の衝突の検査の失敗、spawn_blocking の JoinError）
- 根拠: `not_saved` は理由を捨てる（Task 3 F7・Task 4 F13 の残り。`SettingsWriteError` の Display は秘密を含まない）。`/web-password` の上の失敗は 500 を返すのに読み出しの記録の行を残さない（ログインの口は残す。spec「状態を実際に返した応答のとおりに書く」）
- kind: technical
- 提案: `tracing::warn!(reason = %e)` を出し、500 で返す道でも `forbidden`/500 の行を書く
- 処置: fixed D9
- 判定: ADDRESSED（re-review 5 @a5aade2581f7fc59877526a3af4625e7e94ef23f）
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## final review（56fd1c5..c05e580）

2 回目の whole-branch の final reviewer（SDD の `code-reviewer.md`。package は `.superpowers/sdd/tasks/review-56fd1c5a8..c05e580c9.diff`）の指摘。R22〜R27 の直し（2b27dc1..HEAD）で新しく壊れたもの・ADDRESSED の再発は無い（記録の順・`LOGIN_CHECK` の持ち方・失敗の数え・試験の環境の鍵を確かめた）。Critical・Important は無く、Minor 3 件。Task の ⚠️: task-9「R24 の後に e2e を走らせていない」は解消（6.1 の e2e を取った tree の `messages.ts`・`st59-change-secrets.spec.ts`・`web_session.rs` は HEAD と同じ blob）。Windows の rename と `server.json` の ACL（T2・T7）は未検証のまま、PR の CI の `install-windows` で確かめる。

## R28. 書き戻しに落ちた後、打ち消しの行の INSERT も落ちると、`not_saved` でなく本文 `internal error` の 500 になり、理由もログに出ない（Minor）
- 成果物: `crates/server/src/web_session.rs`（`:949` の `/web-password`、`:1321` の `/collector-token` の `app.access.write(failed()).await?`）
- 根拠: 書き戻しの失敗の後は `?` で抜けるので、記録には `ok` の行だけが残り（値は変わっていない）、画面には「値は変わっていません」が出ない。spec（data-sensitivity「行を書いた後に値を書き戻せなかったとき」）はこの二重の失敗を定めていない。行が多い側への誤りで、害は小さい
- kind: technical
- 提案: 打ち消しの行の書き込みに落ちたときも理由を warn に出し、`not_saved` の 500 を返す。spec は直さない
- 処置: fixed D9
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R29. R27 の試験は書き戻しの失敗の道だけを見ている。ほかの 500 の道で `forbidden`/500 の行と warn が出ることは確かめていない（Minor）
- 成果物: `crates/server/src/st59_tests.rs` / `crates/server/src/web_session.rs`（`:866-934`・`:1290-1302` の hash の失敗・衝突の検査・印の行の INSERT の失敗）
- 根拠: final reviewer の読み。差し込み口が無いので起こしにくい。衝突の検査の `Ok(false)` は `validate_new_web_password` が先に同じ値を断るので通らない守りの道
- kind: technical
- 提案: 起こせる道（例: 印の行の INSERT の失敗）だけでも試験を足すか、起こせない理由を試験の近くに書く
- 処置: fixed D9
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）

## R30. `Ok(Err)` と `Err(JoinError)` の分岐で、warn・行・500 の同じ 3 行が 2 つの口で計 4 回繰り返されている（Minor・nit）
- 成果物: `crates/server/src/web_session.rs`（`:873-882`・`:903-912`・`:1293-1302`）
- 根拠: final reviewer の読み。`not_saved_reason`（`:974`）と同じ形で理由を `String` にまとめられる
- kind: technical
- 提案: 理由を `String` にする小さな関数にまとめ、分岐を 1 つにする
- 処置: fixed D9
- 判定: ADDRESSED（re-review 6 @440e6d137e71fa79fd58c4414d1e5a1600e10d8a）
