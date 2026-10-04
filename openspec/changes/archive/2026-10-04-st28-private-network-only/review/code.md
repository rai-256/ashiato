# ST28 whole-branch review — review/code.md

## final review（7896014..9ba56f7）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-7896014..9ba56f7.diff`、43 commits）。
判定: **With fixes**（Critical 0 / Important 1 / Minor 11 ＋ 推奨 2）。
reviewer がその場で走らせたもの: `check_scenarios.py` OK（492 件担保・人間待ち 0）/ `check-exposure.sh --self-test` OK /
`check-offsite.sh` OK / `cargo test -p ashiato-collector-windows base_url_cleartext_` 2 passed。

Task ごとの記録の ⚠️（Cannot verify from diff）の検算（reviewer）:

| 出所 | 結論 |
|---|---|
| Task 1: `check-panic-log.sh` を新しい DB でアプリの役割で起動 | 解消（`db-roles.sh` → `migrate` → 起動） |
| Task 1: `testdb::pool()` が付与を当てるか | 解消（`MIGRATIONS` の後に `GRANTS`） |
| Task 2: CI の移行の順 / `dist/run.sh` | 成り立つ（rust job は `db-roles` → 試験が移行と付与を当てる / `run.sh` は `stack.sh up`） |
| Task 5: `authorize` を通らない口 | 該当なし（13 本すべて `authorize`。例外は `/healthz` と `/session`、`/selftest/panic` は環境変数で明示したときだけ） |
| Task 6: e2e の通しの順序依存 | 成り立つ（ログアウトの試験は自分の印だけを失効させる。通しは evidence 10.2） |
| Task 7: 7.3 の証跡がいまの head に無い | 成り立つ（collector-windows は 91d4cf8 以降無変更。手元で 2 passed） |
| Task 8: bash のコメントの印 | 検査は通るが Global Constraints から外れる → R12 |
| Task 10: `executor=manual` の証跡 | 影響なし（後で fixer と harness の settle が取り直している） |

fix は 1 回（9ba56f7..3241f02、9 commits）。scoped re-review は 1 回（`re-review-prompt.md`）で **R1〜R14 すべて ADDRESSED、新たな Critical / Important なし**。

## R1. 画面の合言葉が雛形（`change-me-web-password`）のままでもサーバが起動する
- 成果物: `crates/server/src/web_session.rs`（`check_web_password`）/ `.env.example:18` / `design.md` D16（仮）
- 根拠: 雛形の値は 22 文字で 16 文字の下限を通る。DB の合言葉は `db-roles.sh` が `change-me` を拒むが、画面の合言葉には同じ規則が無い。公開リポジトリの文字列でログインでき、Q1 が実質的に外れる
- kind: conflict
- 処置: fixed D16 仮（af9bd4e。`change-me` で始まる値を `reason=placeholder` で拒む。`server_startup_web_password_missing_or_short_refuses` に雛形の 1 例）

## R2. 有効な印を持ったまま再ログインすると `Set-Cookie` が 2 本付き、古い印が勝つ
- 成果物: `crates/server/src/web_session.rs`（`refresh_cookie`）
- 根拠: Task 4 F1 の未対応分。後に付く古い印をブラウザが採り、新しく発行した行が使われない
- kind: technical
- 処置: fixed 4.3 （93dd381。応答が `Set-Cookie` を持てば出し直さない。`web_session_endpoint_relogin_with_cookie_sets_only_new_cookie`）

## R3. 認可の判定が 1 つの要求で最大 3 回走り、ログアウトが割り込むと記録（ok/200）と応答（401）が食い違う
- 成果物: `crates/server/src/access_log.rs` / `web_session.rs` / `lib.rs`
- 根拠: `access_log::middleware`・`authorize`・`refresh_cookie` が別々に DB で判定する（Task 5 F5）
- kind: technical
- 処置: fixed 5.1 （93dd381。最外層で 1 回判定し task-local `AUTHN` に置く。`web_session_authorization_is_decided_once_per_request`）

## R4. `check-exposure.sh` の空振り 2 つ（serve の設定が空でも通る / 画面の待ち受け前に検査する）
- 成果物: `tools/check-exposure.sh` / `tools/stack.sh`
- 根拠: `tailscale serve status --json` が rc=0 で空なら (b) を黙って通す（「読めなければ落ちる」と食い違う）。`stack.sh` は `sleep 2` の後に (a) を走らせる（Task 8 F2 / F3）
- kind: technical
- 処置: fixed 8.1 （e0495d9。空なら落とす・fixture `serve-empty.json`。`ss` で待ち受けを最大 30 秒待つ）

## R5. `check-db-secret.sh` の (a) が `POSTGRES_PASSWORD` の字面しか見ない / (b) のログの置き場が固定
- 成果物: `tools/check-db-secret.sh`
- 根拠: `OWNER_DB_PASSWORD` / `APP_DB_PASSWORD` の字面を拾わない。`/tmp/db-roles-placeholder.log` は並走する worktree と衝突しうる（Task 1 F1）
- kind: technical
- 処置: fixed 1.4 （4b697a1。`(POSTGRES|OWNER_DB|APP_DB)_PASSWORD`、`mktemp` + trap）

## R6. `db_host_is_local` の unix socket の分岐に到達しない
- 成果物: `crates/server/src/net_guard.rs`
- 根拠: sqlx は `?host=/…` を socket として持つので `starts_with('/')` に来ない。拒否側に倒れるが socket の経路が撃たれていない（Task 3 F1）
- kind: technical
- 処置: fixed 3.2 （7f35565。`get_socket().is_some()`。`server_startup_db_not_loopback_host_rules` に socket の例）

## R7. `BIND` の名前を検査と待ち受けで 2 回解決している
- 成果物: `crates/server/src/net_guard.rs` / `lib.rs`
- 根拠: 検査したアドレスと待ち受けるアドレスの一致が保証されない（Task 3 F2）
- kind: technical
- 処置: fixed 3.1 （7f35565。`check_bind` が検査したアドレスを返し、そこへ `TcpListener::bind`）

## R8. `POST /session` の 415 と 500 が読み出しの記録に残らない
- 成果物: `crates/server/src/web_session.rs` / `access_log.rs`
- 根拠: design D8 は `/session` の 3 本を書くと定める
- kind: technical
- 処置: fixed D8 （93dd381。`login_failed` と実際の status で 1 行。`access_log_middleware_session_post_unsupported_and_failed_are_logged`）

## R9. 合言葉の比較が長さの違いで早く返る
- 成果物: `crates/server/src/lib.rs`（`token_matches`）
- 根拠: 画面の合言葉の長さが応答時間から推測されうる
- kind: technical
- 処置: fixed 4.2 （7b018a7。両方を SHA-256 にして `subtle::ct_eq`）

## R10. Android の build 検査は `http://[::1]` を通すが、生成する `domain-config` に `::1` が無い
- 成果物: `collector-android/app/build.gradle.kts` / `tools/check-apk-cleartext.sh`
- 根拠: 組み立ては通り、送信だけが黙って届かなくなる（Task 7 F1）
- kind: technical
- 処置: fixed D13 （d88b6ef。平文の宛先を `localhost` / `127.0.0.1` に揃え、`[::1]` で組み立てが落ちる例を足した）

## R11. `check-exposure.sh --self-test` が同じ Scenario の印を複数持つ
- 成果物: `tools/check-exposure.sh`
- 根拠: 「loopback 以外」3 つ・「通る」2 つ。Global Constraints の 1 Scenario 1 印（review R10）
- kind: technical
- 処置: fixed 8.1 （e0495d9）

## R12. `check-offsite.sh` の印が bash のコメント（`# Scenario:`）
- 成果物: `tools/check-offsite.sh`（同じ違反の `tools/check-db-secret.sh` も）
- 根拠: Global Constraints は bash の印を `echo` と定める（Task 8 ⚠️）
- kind: technical
- 処置: fixed 8.2 （4b697a1）

## R13. 画面がログアウトの失敗を成功に見せ、429・5xx・通信の失敗でも「合言葉が違います」と出す
- 成果物: `web/src/session.tsx`
- 根拠: Task 6 F1 / F2
- kind: technical
- 処置: fixed 6.2 （c70dccb。ログアウトは 2xx のときだけ入力欄へ。ログインの文言を 401 / 429 / それ以外に分けた。vitest 5 本）

## R14. 書き込みの CSRF の守りが SameSite=Strict と JSON の本文だけに依り、`/stays/rebuild` は本文の型を見ない
- 成果物: `design.md` D12
- 根拠: 同じ機械名の別の口からは text/plain の POST で届きうる。D12 の「同居させない」が前提
- kind: technical
- 処置: fixed D12 （3241f02。反転条件を D12 に書いた。`/stays/rebuild` は空の本文を受ける先行の約束があるのでコードは変えない）

## scoped re-review（9ba56f7..3241f02）の残り（park）

re-review の新たな指摘は Minor 2 件のみ。2 回目の fix は無い（SDD）ので ruling つきで残す:

- `tools/check-apk-cleartext.sh` の `[::1]` の例は落ちた理由（`暗号化されていない`）を確かめない — Ruling: 残す — task 名は実在し空振りではない。別の理由で落ちても通るのは検査の緩さで、誤りの経路ではない — 誤りなら `[::1]` の拒否が回帰しても気付くのが遅れる
- R8 の試験は Scenario の印を持たない — Ruling: 残す — D8 の細部で、spec の Scenario ではない（意図をコメントに書いてある）— 費用なし

## 推奨（PR 本文の「仮で決めたもの」に書く）

- D17（仮）の上限は機械全体で 1 つなので、網の内側の 1 台が 1 分に 10 回失敗し続けると本人もその 1 分は 429 になる

## code-verify（2560aa5。独立検証 2026-10-01）

席: `code-verify`。読んだ印象ではなく、走らせた結果だけを書く。値を変えて試すもの（変異）は使い捨ての worktree（`/tmp/cv28-mut`。終わったら消した）でやり、作業ツリーのコードは触っていない。
DB は `docker compose`（この worktree の `ashiato2-st28`、`127.0.0.1:55433`）。

**申告: 37/37 `[x]`**（未完了 0）。検証コマンドは tasks.md の各項目の本文に書かれているもの。

| 項目 | 実行したコマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `tools/db-roles.sh && tools/db-roles.sh` / `core` の表で所有者が `ashiato_owner` でないものの数 | 一致（rc=0 / 0 件） |
| 1.2・1.4 | `tools/check-db-secret.sh`（全体） | 一致（rc=0） |
| 1.3 | `cargo test --workspace`（`.env` を読み込んで）/ `git diff --exit-code origin/main -- dedup/registry/drops_tests.rs` | 一致（459 passed・0 failed / rc=0。origin/main を fetch して遅れ 0） |
| 2.1〜2.5, 3.1〜3.3, 4.1〜4.4, 5.1〜5.3 | `CT <絞り込み>` 14 本を 1 本ずつ | 一致（どれも 1 本以上 passed: 1/1/1/1/3/6/2/1/2/10/4/8/1/1）。接頭辞 5 つで `--list` に出る 40 本は main に無い名前だけ |
| 2.2 | `tools/smoke.sh` / `STACK_RESET=1 bash tools/stack.sh up --check-only` | 一致（rc=0 / rc=0。smoke の「2b」が OK） |
| 2.6・5.2 | `tools/check-immutable.sh` | 一致（rc=0。`OK app-role` 3 行・`OK access_log` 3 行）。**注**: この台本は終わりに `docker compose down -v` で DB を消す（main からある作り） |
| 3.4・6.1・9.1・9.2 | tasks に書かれた `grep` | 一致（rc=0） |
| 4.5 | `tools/check-openapi.sh` | 一致（rc=0） |
| 4.1 | `tools/check-migrations.sh` | 一致（rc=0） |
| 6.2 | `npx vitest run src/__tests__/session.test.ts` / `npm run lint && npm run build` | 一致（rc=0） |
| 6.1・6.3・6.4・10.2 | `npm test`（147 passed）/ `npx playwright test`（17 passed） | 一致（rc=0） |
| 7.1 | `tools/check-apk-cleartext.sh` | 一致（rc=0。Scenario の印 2 つ） |
| 7.2 | `tools/android-emulator.sh` | 一致（rc=0）。証跡（evidence.jsonl）は 91d4cf8 のもので、その後 d88b6ef で `build.gradle.kts` が変わっていたので、**いまの head で走らせ直した** |
| 7.3 | `cargo test -p ashiato-collector-windows base_url_cleartext_` | 一致（`--workspace` の中で passed） |
| 8.1 | `tools/check-exposure.sh --self-test` / `tools/check-exposure.sh`（実機） / `tools/check-private.sh` | 一致（rc=0 / rc=0 / rc=0） |
| 8.2 | `tools/check-offsite.sh` / `--self-test` | 一致（rc=0 / rc=0） |
| 8.3 | `tools/check-private.sh --self-test` | 一致（rc=0） |
| 10.1 | `python3 scripts/check_scenarios.py . st28-private-network-only` | 一致（OK。この change の 59 本はどれも印がちょうど 1 つ） |
| 10.2 | `cargo clippy --all-targets -- -D warnings` / `tools/check-licenses.sh` | 一致（rc=0 / rc=0） |
| 10.3 | `python3 scripts/review_triage.py . st28-private-network-only` | 一致（この節を足す前の時点で rc=0） |

**手 1（固定値を独立に計算し直す）: 不一致は無かった。** テストに期待値をそのまま書いたハッシュは無い。実物は 1 回ログインして DB の行を読み、Python（`hmac` / `hashlib` / `base64`）で計算し直した:
`secret_tag` = HMAC-SHA256(鍵 `API_TOKEN`、本文 `ashiato-web-session-v1` ‖ `WEB_PASSWORD`) が一致した（design D3）。`token_sha256 = sha256(cookie の値)` でその行が引けた。印は 43 文字で、復号すると 32 バイトだった。

**手 2（守りをわざと壊す）**: 次の 5 つは、守りを潰すと実際に落ちた。
- compose に字面の合言葉を戻す → `check-db-secret.sh --only compose,testdb` が rc=1
- workflow に `postgres://…:<字面>@` を書く → rc=1
- サーバに `ureq` を足す → `check-offsite.sh` が rc=1（`NG ureq`）
- `grants.sql` の REVOKE を `GRANT TRUNCATE` に変える → `check-immutable.sh` が rc=1（`NG app-role TRUNCATE`）、`app_role_privileges_reach_every_table` と `app_role_cannot_bypass_gate` が FAILED
- `THROTTLE_MAX_FAILURES` を 11 / `MIN_WEB_PASSWORD_LEN` を 12 に変える → 試験が FAILED

検査の範囲から漏れていたものは R15・R17・R19 に書いた。

## R15. 本人の決定（Q6 の「期限なし」）と、仮決めの値（D17 の 60 秒と 1 秒・D18 の 400 日）は、値を変えても全部のテストが通る
- 成果物: crates/server/src/lib.rs:2027-2033（`WEB_SESSION_MAX_AGE_DAYS` が無いときの既定 `0`）/ crates/server/src/web_session.rs:28,34,145（`COOKIE_MAX_AGE_SECS`・`THROTTLE_WINDOW`・`failure_delay`）/ crates/server/src/web_session_tests.rs
- 根拠: 使い捨ての worktree で 1 つずつ書き換え、`cargo test -p ashiato-server` を全件（364 + 7）走らせた結果:
  - `Err(_) => 0` を `30` に → **364 passed / 7 passed**（全部通る）
  - `THROTTLE_WINDOW` の 60 秒を 3600 秒に → 全部通る
  - `failure_delay` の 1 秒を `Duration::ZERO` に → 全部通る
  - `COOKIE_MAX_AGE_SECS` の 400 日を 2 日に → 全部通る
  - 比べると、`THROTTLE_MAX_FAILURES` の 10→11 は 2 本、`MIN_WEB_PASSWORD_LEN` の 16→12 は 1 本が FAILED になった（この 2 つは固定されている）

  Scenario「既定では日が経ってもログインは切れない」は「期限を設定せずに」と言っている。一方で試験（`web_session_lifetime_default_never_expires`）は `app_with(&pw, 0)` で `0` をはっきり渡しており、設定が無いときの既定の値を通っていない。主張の層（設定しない）とテストの層（0 を渡す）が食い違っている。`for_test` も `0` を直書きしている（lib.rs:262）。
- kind: technical
- 提案: 環境から読む部分を `fn session_max_age_days(var: Option<&str>)` に切り出し、`None` → `0` を Scenario の試験で確かめる。窓（60 秒が過ぎたら 429 が解ける）と既定の待ち（`WebLogin::new` の待ちが 1 秒）と cookie の `Max-Age == 34560000` は、それぞれ 1 本ずつ assert で固定する
- 処置: fixed D18 （`web_session::session_max_age_days(None)` を切り出して Scenario の試験がそれを通る。窓 60 秒・待ち 1 秒・`Max-Age` 400 日を単体の試験で固定。4 つの変異はどれも FAILED になった）

## R16. アプリの役割に所有者の役割を付けると（`GRANT ashiato_owner TO ashiato_app`）、起動時の自己検査を通ったまま門のトリガ 7 本を止められる
- 成果物: crates/server/src/lib.rs:183-200（`check_db_role`）/ tools/db-roles.sh
- 根拠: この worktree の DB で `GRANT ashiato_owner TO ashiato_app` を当てて確かめた（最後に REVOKE で戻し、0 件になったことを見た）:
  - アプリの接続で `BEGIN; ALTER TABLE core.event DISABLE TRIGGER USER; …; ROLLBACK` を実行。付与の前は `ERROR: must be owner of table event`、**付与の後は `disabled=7`**（ROLLBACK したので何も残っていない）
  - 同じ状態でサーバ（`DATABASE_URL` はアプリの役割）を起動すると **`/healthz` が 200** を返した。`check_db_role` は `rolsuper` と `tableowner = current_user` しか見ず、役割を継承しているかを見ていない
  - `tools/db-roles.sh` を当て直しても、この付与は残った（`pg_auth_members` が 1 件のまま。台本に `REVOKE` / `pg_auth_members` を扱う所が無い）

  R103（記録の門を外せる）が、別の経路から開いたままになっている。権限の不足に当たった人が「とりあえず所有者を付ける」と、黙って再現する。spec の「アプリの接続から、記録の門…を外す設定・表の定義の変更・表の切り詰めを拒む」に反する。
- kind: technical
- 提案: `check_db_role` に「`core` のどれかの表の所有者に対して `pg_has_role(current_user, tableowner, 'USAGE')` が真なら拒否する」を足す（理由の種別は `owner_member`）。`db-roles.sh` は `ashiato_app` が持つ役割の付与をすべて REVOKE する。結合の試験を 1 本足す
- 処置: fixed D4 （`check_db_role` に `pg_has_role(current_user, tableowner, 'MEMBER')` を足し `reason=owner_member`。`db-roles.sh` が `ashiato_app` の役割の付与をすべて REVOKE する。`server_startup_refuses_privileged_role` に使い捨ての役割で 1 例。付与を当てて `db-roles.sh` を 2 回走らせると 1 → 0 件）

## R17. `check-exposure.sh` を単独で走らせると `.env` を読まないので、見る port がサーバの既定と食い違い、平文の口を見落とす（手順書は単独で走らせるよう指示している）
- 成果物: tools/check-exposure.sh:19-20 / docs/network.md:18,55
- 根拠:
  - 台本の既定は `BIND=127.0.0.1:18787`・DB は `55432` の決め打ちで、`.env` を読まない。サーバの既定は `127.0.0.1:8787`（lib.rs:2035）
  - `tailscale serve` が `:8787` を平文で中継している fixture（`TCP.8787.HTTP` / Proxy `http://127.0.0.1:8787`）に対して、`env -u BIND tools/check-exposure.sh` は **`OK 網の外に開いている口は無い`・rc=0**。同じ入力で `BIND=127.0.0.1:8787` を渡すと `NG 本システムの口を平文で網へ出している port=8787`・rc=1 になる
  - `0.0.0.0:55433`（この worktree の `docker-compose.override.yml` が使っている DB の port）で待ち受けている ss の出力でも rc=0 になる

  `tools/stack.sh` は `.env` を export してから呼ぶので、そちらは正しい port を見る。漏れるのは、`docs/network.md` §1 と §6 の 4 が本人に走らせる単独の実行のほう。
- kind: technical
- 提案: 台本の頭で `.env` を読む（`set -a; . ./.env` を、`BIND` / `WEB_PORT` がまだ設定されていないときだけ）。DB の port は `docker compose port db 5432` から取る。どちらも取れなければ落とす
- 処置: fixed 8.1 （渡されていない `BIND` / `WEB_PORT` / `DEV_WEB_PORT` を `.env` から補い、無ければサーバの既定 `127.0.0.1:8787`。DB の port は `docker compose config` の公開の設定から取り、取れなければ落ちる。`0.0.0.0:55433` の ss で rc=1）

## R18. `server_startup_bind_default_is_loopback_only` は 127.0.0.1:8787 が空いていないと落ちる（worktree を並べて回すと揺れる）
- 成果物: crates/server/tests/server_startup.rs:141-172
- 根拠: `python3` で 127.0.0.1:8787 を塞いでから `cargo test --test server_startup server_startup_bind_default` を走らせると、`サーバが待ち受ける前に終わった: Error: Address already in use (os error 98)` で FAILED。この試験は `env_clear()` で `BIND` を外し、既定の固定 port に待ち受ける。この repo は worktree を並べて回すので（hx dev は 8 run）、別の worktree のサーバが 8787 を使っていればこの試験が落ちる
- kind: technical
- 提案: 既定の port を空いているかどうかで決めない。代わりに「既定のアドレスは loopback」を純関数（`default_bind()` を返す関数と `bind_allowed`）で固定し、結合の試験では `BIND=localhost:0` で待ち受けの口がすべて loopback になることを見る。どうしても固定 port で撃つなら、使用中のときは理由を書いて飛ばす
- 処置: fixed 3.1 （既定を `net_guard::DEFAULT_BIND` に置き、試験はまずそれが loopback であることを見る。8787 が使用中なら理由を出して起動の確かめを飛ばす。塞いだ状態で passed・skip の行が出た）

## R19. `check-db-secret.sh` は `PGPASSWORD=<字面>` を拾わない
- 成果物: tools/check-db-secret.sh:35-36
- 根拠: 使い捨ての worktree で `tools/dev.sh` に `PGPASSWORD=ashiato psql -h 127.0.0.1 -U ashiato …` を足しても、`check-db-secret.sh --only tools` は rc=0（`Scenario: 配布物に DB の合言葉の固定値が無い` を出す）。拾うのは `(POSTGRES|OWNER_DB|APP_DB)_PASSWORD` と `postgres://役割:値@` の 2 つの形だけ。いま repo に `PGPASSWORD` を使う所は無いので（`git grep` で 0 件）、実害はまだ無い
- kind: technical
- 提案: `lit_env` の名前に `PGPASSWORD` を足す。libpq の key=value の形（`password=<字面>`）も足す
- 処置: fixed 1.4 （`PGPASSWORD` と libpq の `password=<字面>` を足した。字面の 2 例は rc=1、`$` の参照の 2 例は rc=0）

**手 3（Scenario とテストの突き合わせ）**: `check_scenarios.py` は OK で、59 本はどれも印がちょうど 1 つ。印の先のテストを 1 本ずつ読んだ。主張の層とテストの層が食い違っていたのは R15 の 1 件（「期限を設定せずに」と言いながら 0 を直に渡している）だけだった。
ほかに読んで確かめたもの: e2e の「1 件も返らない」（`/api/` の GET がすべて 401 で、行の要素が 0）、「スクリプトから読めない」（`document.cookie`）、`offsite` の 5 本、`server_startup` の起動の結合（実際のプロセスの終了コードと `kind=`）、`app_role_*`（`pg_tables` から表を全部引いている）。

**手 5（`[x]` と実体）**: 上の表のとおり、37 項目の検証コマンドはどれも存在し、rc=0 だった。挙げられていて実在しないテスト名やターゲットは無かった（`--test server_startup` は `crates/server/tests/server_startup.rs` に実在する）。7.2 は証跡が古い head のものだったので、いまの head で取り直して rc=0 を確かめた。

**手 6（隙間）**: 見つけたのは R16。次のものは確かめたが、捨てられたり取りこぼしたりする経路ではなかった:
- 移行の手順 4（http の口を消す）と手順 5（APK を入れ直す）の間に、電話が送れない時間ができる。この間、電話は送れなかった分を積んでおき、上限は 90 日・2 GB（`Retention.kt:16-18`）なので、手順の間で捨てられることは無い
- 読み出しの記録について、実際のサーバで次を確かめた。資格情報の無い `/events`（401）、偽の合言葉での `/ingest`（401）、合言葉での `POST /stays/rebuild`・`HEAD /events`・`OPTIONS /events` は、どれも 1 求めにつき 1 行増えた。存在しない口（404）は 0 行だった。404 は何も読んでいないので、指摘にしない
- C-02 の宛先の検査は、`user@host`・`#`・`?` を使った回り込みに対して、正しく host を取り出していた。Windows 側の `ASHIATO_BASE_URL` は User / Machine のどちらにも設定が無かったので、本人の環境での確かめはできなかった

## code-verify の fix の scoped re-review（2560aa5..441d16b）

fix は 1 回（086eec2 / 441d16b）。scoped re-review は 1 回（`re-review-prompt.md`）で **R15〜R19 すべて ADDRESSED、新たな Critical / Important なし**。
Minor 2 件は 2 回目の fix が無い（SDD）ので ruling つきで残す:

- R18 の試験は 8787 が使用中だと既定での起動の確かめを stderr の 1 行で飛ばす — Ruling: 残す — 既定が loopback であることは同じ試験の前半で必ず確かめ、CI の runner では 8787 は空いている — 誤りなら並べた worktree の手元でだけ起動の確かめが抜ける
- R19 の新しい形（`PGPASSWORD` / `password=`）に自己検査が無い — Ruling: 残す — 変異の 4 例を手で確かめた（cv-fix-report）。`check-db-secret.sh` は先行から自己検査を持たない作り — 誤りなら正規表現の回帰に気付くのが遅れる
