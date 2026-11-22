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

## R2. Scenario「変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない」の担保が無い（印の先は別の主張を見ている。印の無い試験も含め、サーバの単体は壊した実装で全部通る）
- 成果物: `crates/server/src/st59_tests.rs:137-158`（印の先 `st59_web_password_rechecks_secrets_after_serialization_permit` は、変える口の照合の間に共有の値を差し替えて 400 `same_as_api_token` を見る。ログインの印は見ていない）/ `:1256-1282`（印の無い `st59_swap_login_cookie_uses_the_old_tag_and_is_revoked_after_swap`）/ tasks 1.1・3.1
- 根拠: ログインの口の印の行を、照合した組の `tag` ではなく書く時点の組の `tag` で書く変異（`web_session.rs:1408` の `.bind(secrets.tag())` → `.bind(app.secrets_snapshot().tag().to_vec())`）→ `cargo test -p ashiato-server st59_` が **38 passed / 0 failed、rc=0**。印の無い `st59_swap_login_cookie_…` だけを 5 回走らせても 5 回とも `ok`。
  この変異と、R3・R5 の変異を同時に入れた複製で `cargo test -p ashiato-server --lib` → **2035 passed / 0 failed / 3 ignored、rc=0**（サーバの単体のどれも落とさない）。
  `#[tokio::test]` は 1 本の thread で動くので、`check_succeeded` の通知から印の行の `bind` までの間に試験の差し替えが割り込めない。ログインは照合の後に印を放してから印の行を書く（`check_given_password_with_secrets` の中の印は関数を出ると放す）ので、本番では変える口がこの間に割り込める
- kind: technical
- 提案: `flavor = "multi_thread"` にし、照合の後・印の行の INSERT の前で止める差し込み口（`#[cfg(test)]`）を足して、そこで本物の `POST /web-password` を通す試験に、この Scenario の印を移す。上の変異で落ちることを確かめる

## R3. Scenario「新しい合言葉が検査に当たれば、何も変わらない」の理由の文字列と 15 文字の境界は、サーバの単体のどれも見ていない
- 成果物: `crates/server/src/st59_tests.rs:950-983`（`st59_web_password_handler_applies_all_five_validation_rules` は状態の 400 だけを見て本文を読まない。短すぎの入力は `"short"`（5 文字））/ `web_session.rs` の `validate_new_web_password` / spec「どれも 400 で、本文の理由はそれぞれ `too_short`・`placeholder`・…」「15 文字の合言葉」
- 根拠: `validate_new_web_password` の 1 つ目の検査を `if new_password.chars().count() < 6 { return Err("mismatch");` に変える変異（下限 16 → 6、理由 `too_short` → `mismatch`）→ `cargo test -p ashiato-server st59_` が **38 passed、rc=0**（R2・R5 の変異と同時に入れた `--lib` 全体も 2035 passed、rc=0）。e2e にも理由ごとの文言を見る試験は無い（`web/e2e/st59-change-secrets.spec.ts` は 400 を送らない）。`grep -c 'too_short\|placeholder\|same_as_session_key\|"mismatch"' crates/server/src/st59_tests.rs` → `0`（ST59 の試験に理由の文字列は 1 件も無い）。
  画面は理由ごとに別の文言（D11）を出すので、理由を取り違えると、本人に違う直し方を示す
- kind: technical
- 提案: 5 つの入力（短すぎは **15 文字**ちょうど）ごとに本文 `{"error": "<理由>"}` を `assert_eq!` で固定し、16 文字は 400 にならないことも 1 件見る

## R4. Scenario「変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない」の試験は、設定のファイルを読まずに新しい合言葉を直接渡して組み立てている
- 成果物: `crates/server/src/st59_tests.rs:1187-1221`（`let rebuilt = app.with_web_login("new-web-password-012345", 0);`）/ tasks 3.1「書き直した `server.json` から組み立て直した `App` で」
- 根拠: 変える口の書き戻しを消す変異（`if crate::service::replace_setting("WEB_PASSWORD_VERIFIER", &verifier).is_err() {` → `if false {`）→ `st59_web_password_` と `st59_both_` を走らせて **この試験は `ok`**（落ちたのは `st59_web_password_handler_updates_only_the_verifier_key` と `st59_both_write_failure_returns_not_saved` の 2 本だけ）。
  収集アプリの値の側（`st59_collector_token_uses_new_token_after_rebuilding_app`）は `load_settings` で読み直しているのに、合言葉の側は読んでいない。
  この Scenario の起動し直しを本物で見るのは `install-check.sh` だけで、それは PR の CI の `install-windows` まで走らない（手 6）
- kind: technical
- 提案: `service::load_settings` で書き直した `server.json` を読み、その `WEB_PASSWORD_VERIFIER` で `WebLogin::new_with_verifier` を組んだ `App` で、前の合言葉は 401・新しい合言葉は 204 を見る（`st59_collector_token_uses_new_token_after_rebuilding_app` と同じ形）

## R5. Scenario「…値は、記録にもログにも応答の見出しにも残らない」の「ログ」は、サーバの単体も `check-log-private.sh` も見ていない
- 成果物: `crates/server/src/st59_tests.rs:738-805`（`st59_both_change_values_are_not_logged` は読み出しの記録の行と応答の見出しだけを見る。`:1223-1243` も読み出しの記録だけ）/ `tools/check-log-private.sh`
- 根拠: `web_password_post_inner` の照合の前に `tracing::warn!(new = %request.new_password, current = %request.password, "st59 mutation");` を足す変異 → `cargo test -p ashiato-server st59_both_change_values_are_not_logged` **ok、rc=0** / `tools/check-log-private.sh` **rc=0（`check-log-private: OK`）** / R2・R3 の変異と同時に入れた `--lib` 全体も 2035 passed、rc=0。
  tasks 3.1 の本文「入れた合言葉と照らす値がサーバのログと読み出しの記録に無い」も、ログの側は確かめていない
- kind: technical
- 提案: 試験の中で `tracing` の出力を集め（既存の試験にログを捕まえる部品があればそれを使う）、2 つの口を成功・失敗で通した後に、今の合言葉・新しい合言葉・新しい照らす値・新旧の収集アプリの値が無いことを見る

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

範囲外: `access_log::middleware` の素通しの一覧（`access_log.rs:166`）と `collector_scope::credential_exempt` は、別々の一覧を手で揃えている。今回も 2 か所に同じ口を足した（既存の形。R 番号は振らない）
