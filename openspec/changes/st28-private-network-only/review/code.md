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
