# ST29 code review

## code_verify（独立検証。2026-10-25。HEAD 0af733c9）

席: code_verify（実装していない者が、実行して確かめる）。入力は `openspec/changes/st29-satellite-credentials/`（deep / specs / design / tasks）と作業ツリー。
コードは触っていない。壊す実験は作業ツリーで一時的に行い、毎回 `git checkout --` で戻した（各回の後の `git status --porcelain` はこのファイルの 1 行だけ）。
後ろの「final review」節（R1・R2・Minor・re-review）は別の席の記録で、そのまま残す。番号がぶつからないよう、この節は R3 から続ける。

### 申告: 23/23（tasks.md の `- [x]`）・検証コマンド一覧と実測

| # | 申告の検証 | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` rc=0・`CT satellite_migration 2` | rc=0 / 4 本 ok | 一致 |
| 1.2 | `tools/check-immutable.sh \| tee` rc=0 かつ出力に satellite | rc=0・`satellite` 27 行・`書き換え禁止 OK` | 一致 |
| 2.1 | `CT satellite::tests 5` | 6 本 ok | 一致 |
| 2.2 | `CT satellite_form 2` | 3 本 ok | 一致 |
| 3.1 | `CT satellite_authn 3` | 3 本 ok | 一致 |
| 3.2 | `CT access_log 9`・`CT satellite_access 5`・`CT access_log_secrets_satellite 1` | 11 / 5 / 1 本 ok | 一致 |
| 4.1 | `CT satellite_register 9` | 10 本 ok | 一致 |
| 4.2 | `CT satellite_admin_only 2` | 2 本 ok | 一致 |
| 4.3 | `CT satellite_builtin 1` | 1 本 ok | 一致 |
| 5.1 | `CT satellite_credential_issue 6` | 6 本 ok | 一致 |
| 5.2 | `CT satellite_credential_revoke 4`・`CT satellite_admin_only 2` | 4 / 2 本 ok | 一致 |
| 5.3 | `tools/check-log-private.sh` rc=0 | rc=0 | 一致 |
| 6.1 | `CT every_route_refuses_satellite 1`＋変異で落ちる | 1 本 ok。変異は手 2 の M1・M2（片方ずつでも落ちる） | 一致（ただし R5 で 3 回に 1 回落ちる） |
| 6.2 | `CT exempt_routes_ignore_satellite 1` | 1 本 ok | 一致 |
| 7.1 | `CT export_satellite 3`・`CT export_ 60`・`grep -c '"satellites.jsonl"'` ≥1 | 3 本 ok / 127 本 ok・1 本 FAILED（下の注）/ 3 | 一致（注） |
| 8.1 | `tools/check-satellite-boundary.sh` rc=0・`--self-test` rc=0 | rc=0 / rc=0 | 一致 |
| 9.1 | `cd web && npm run lint && npm run test -- satellites` | lint rc=0 / vitest 8 passed | 一致 |
| 9.2 | `ET 'Scenario' 8` | `--list` 8 本・`STACK_RESET=1 npm run test:e2e -- settings-satellites --grep Scenario` rc=0（8 passed） | 一致 |
| 10.1 | `tools/check-openapi.sh` rc=0 | rc=0 | 一致 |
| 10.2 | `tools/smoke.sh` rc=0 | rc=0（ST29 の節の 4 つの OK）。別の 1 回は ST12 の書庫の節（`書庫の最終日が /archives/status に出ない`）で rc=1 —— ST29 の節より前で、ST29 は触っていない | 一致 |
| 10.3 | check_scenarios / review_triage / check_chain rc=0 | 0 / 0（この節を書く前。書いた後は、処置の無い R3〜R5 の 3 件だけで `triage: FAIL (3 件)`。処置は呼び出し元が付ける）/ 0 | 一致 |
| 10.4 | fmt / clippy / test --workspace / web 一式 / immutable / log-private / boundary / openapi / smoke | fmt・clippy（`-D warnings`）rc=0 / server 1250 passed・1 failed（注）・他の crate 283 passed / lint・vitest 470 passed・build・e2e rc=0 / 検査 4 本と smoke rc=0 | 一致（注・R5） |
| 10.5 | 印の grep ≥1・`git diff main -- lib.rs` の `-…Caller::ApiToken` が 0 | 1 / 0（origin/main とも merge-base 4ee5f442 とも 0） | 一致 |

CT の本数は `scripts/quiet-run unit -- bash -o pipefail -c 'cargo test -p ashiato-server 2>&1 | tee /tmp/st29-ct-all.log'`（1 回の全走。4412 秒）から、絞り込みごとに `^test <…絞り込み…> ... ok` を数えた。

注（7.1 / 10.4 の 1 本）: 全走で `export_job_tests::export_status_leftover_digest_is_not_reused_after_the_file_changes` が `internal error`（500）で落ちた。
同じ DB（全走の後）で単独に走らせ直すと `書き出しが終わらない`（5 分待ち）で落ち、DB を作り直して単独で走らせると 2.56 秒で緑。
全走の間、この席が同じ DB に変異の試験を並べて走らせていた（R5 の TRUNCATE を含む）ので、席の側の負荷と溜まった行が重なったものと読む。
ST33 の試験で、この branch の差分（merge-base から）は待ちの回数を変えていない（084d9b7a で元へ戻している）。指摘にしない。

### 手 1: 固定値を独立に再計算する

- 識別子の正規表現: `identifier_check_agrees_with_migration_regex_in_db` は移行の字面から取り出した正規表現を PostgreSQL の `~` にかけ、Rust の手書きの判定と 18 件で突き合わせている（実装の写しではない）。一致。
- 資格情報の値の形（`ashsat_<識別子>_` + 43 文字・`[A-Za-z0-9_-]`）は、発行の応答の値を長さと文字種で見ている。43 文字は 32 バイトの `=` なし base64 の長さ（⌈32×4/3⌉ = 43）。一致。
- ハッシュ: 期待値を実装の外で作っているテストが無い —— R4。

### 手 2: ガードをわざと壊す

| 壊したもの | 実行 | 結果 |
|---|---|---|
| M1: `crates/server/src/access_log.rs` の middleware の衛星の 403 の分岐を消す | `cargo test -p ashiato-server satellite_` | rc=101。`every_route_refuses_satellite` が `POST /attributes/kinds`（`left: 422 right: 403`）で落ちた |
| M2: `crates/server/src/lib.rs` の `authorize` の `Satellite` → 403 の腕を消す | 同上 | rc=101。`every_route_refuses_satellite`（`GET /session` 200）と `satellite_access_session_state_is_forbidden_and_logged`（200）が落ちた |
| M3: `satellites_delete` の資格情報の `revoked_at` を `$2 + interval '1 hour'` に | 同上 | `satellite_register_removed_identifier_returns_same_row` が「外した時刻で取り消される」で落ちた |
| M4: `web_session::sha256` を `Sha256::digest(format!("{s}!"))` に | `satellite_credential` / `satellite_authn` / `satellite_access` / `every_route` / `web_session` の絞り込み | **全部 rc=0**（10 / 3 / 5 / 1 / 19 passed）—— R4 |
| `crates/server/Cargo.toml` に `libloading = "0.8"` | `tools/check-satellite-boundary.sh` | rc=1。`NG 禁じる部品が依存にある: libloading` |
| 移行の `satellite_credential_no_truncate` の CREATE TRIGGER を消す | `tools/check-immutable.sh` | rc=1。`NG satellite_credential TRUNCATE が通ってしまった`・`書き換え禁止 NG` |
| `credentials_post` に `tracing::warn!("verify-mutation {value}")` | `tools/smoke.sh` | rc=1。`ログに資格情報の形がある` |

検査の外側も見た: `crates/server` は workspace の別の crate に依存しない（`path =` が無い）ので、ソースの grep が `crates/server/src` だけなのは抜けではない。CI は `ci.yml:88-89` で 2 本とも走らせる。

### 手 3: Scenario と test

`python3 scripts/check_scenarios.py . st29-satellite-credentials` → `scenarios: OK` rc=0。この change の Scenario 61 本（`specs/*/spec.md` の `#### Scenario:`）は 1 本ずつ grep して、全部に印がある。
印の先を読み、主張の階層と観測の階層を比べた。ずれは見つからなかった:
`衛星の資格情報で送った記録は取り込まれない`（本物の合言葉で同じ本文が 200 になることを先に確かめてから、3 表の件数を比べる）・
`読み出しの記録には中身も合言葉も残らない`（表の全列の `::text` で、値・識別子・合言葉・印を否定）・
`衛星の資格情報は書き出しに入らない`（zip の全 entry で、16 進と `\x` 付きの両方の形を否定し、README の「入らないもの」も見る）・
`衛星の資格情報は日が経っても切れない`（400 日後に 403 を断言）・
`外した衛星と取り消した資格情報の行は消えずに残る`（bash は B の時刻を自分で書くが、口が外した時刻を付けることは M3 で落ちる Rust の試験が持つ）。
全 route の試験は、呼ぶ method を契約から取り、`lib.rs` とは path の集合だけを突き合わせる。python で `lib.rs` の（method, path）と `docs/openapi.json` を比べると、差は契約に載せない `GET /healthz` だけ（37 対 36）。
middleware がどの method にも当たるので、いまは漏れが無い。指摘にしない。
`check_scenarios.py` の warn に、この Story のファイルの印が 1 件ある: `web/src/__tests__/satellites.test.tsx` の `Scenario: 画面の文言はsatellite.rsの定数に合わせたことをテストで確かめる（D5）`。spec に無い名前（Global Constraints の「印の名前は spec と一字一句」に外れる）。warn で、担保の数には効かないので R にしない。

### 手 4: 本人の決定が test で固定されているか

deep.md の Q1〜Q8 と C1〜C15 から値を取り、値を書き換えたら落ちるテストを探した。
Q2 続き（番号と登録の時刻が同じ）/ Q3 外せない（409 `builtin`）/ Q4 値の形（頭と 43 文字）/ Q5 期限なし（400 日後に 403）/ Q6 403（全 route）/ Q7 衛星向けの口を置かない（置けば全 route の試験が 403 でないことで落ちる）/
C1 64 文字（64 を通し 65 を断る）/ C3 本体の前置き / C4 32 バイト（43 文字）/ C7 書き出し / C8 資格情報の要らない口 / C11 依存の検査 / C13 画面のログインだけ /
D15 の 40 文字（衛星の名前と資格情報の名前の 41 文字）には、どれも落ちるテストがある。
C4 の「ハッシュだけを持つ」のハッシュの関数（design D6 の値全体の SHA-256）は、変えても全部通る —— R4。

### 手 5: tasks の `[x]` と実体

23 件とも、挙がったテスト名・絞り込み・コマンドが実在し、上の表のとおり走った。`cargo test --test <名前>` 型の検証は無い。
件数の下限はどれも満たす。ただし 6.1 の判定のテスト（Story の完了の判定）は、壊していない状態でも 3 回に 1 回落ちる —— R5。

### 手 6: 隙間

- 起動のたびの全版の当て直しで、読み出しの記録の CHECK を付け直す。後の Story が種類を足すと、次の起動でサーバが上がらない（R3。scratch DB で再現）
- 発行の応答が届かなかった（網が切れた）とき: 値は戻らないが、行は一覧に「まだ使われていない」で出て取り消せる（spec の「失くした値は取り消して発行し直す」の範囲）。指摘にしない
- 時計が戻る: `last_used_at` が前より古い時刻で上書きされるだけで、記録は失われない。指摘にしない
- 資格情報の判定の UPDATE が失敗すると（R5 の deadlock で実測）、その求めは 403 でなく 500 になる。閉じている側なので指摘にしない

### 手 7: 該当なし（確かめた範囲: `python3 scripts/plan_fix.py pending openspec/changes/st29-satellite-credentials` が `[]`。correction-review.md は書いていない）

## R3. 起動のたびに読み出しの記録の CHECK を付け直すので、後の Story が資格情報の種類か結果を 1 つ足して行を書くと、次の起動でサーバが上がらない
- 成果物: migrations/202610241200_satellite_credentials.sql:66-71 / crates/server/src/lib.rs:169（`migrate()` は起動のたびに全版を当てる）
- 根拠: scratch DB（`st29_verify_scratch`。確かめた後に消した）に ORDER の全版を当てた。後の版を模して `access_log_credential_check` を `…'none','future_kind'` で付け直し、`credential = 'future_kind'` の行を 1 行 INSERT した。そこへ ST29 の版を当て直すと `ERROR: check constraint "access_log_credential_check" of relation "access_log" is violated by some row`（psql rc=3。起動の `migrate()` はここで止まる）。`access_log_outcome_check` も同じ DROP / ADD の形。`DROP CONSTRAINT IF EXISTS` を使う版は `migrations/*.sql`（down を除く）でこの 1 本だけ（`grep -ln`）。前の版 `202609290900_access_control.sql` は CHECK を `CREATE TABLE IF NOT EXISTS` の中に書いているので、当て直しても付け直さない。`satellite_migration_applied_twice_keeps_two_builtins` は後の種類の行が無い空の DB に当てるので、これを拾わない。呼び出し元の種類か結果を足すのは承認（ST38）・衛星の読み出し（ST45）・合言葉の範囲（ST44）の見込みで、そのときは merge 済みの ST29 の版を書き換えるしかない
- kind: technical
- 提案: 制約が無いときだけ足す形にする（`DO $$ … IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = …) THEN ALTER TABLE … ADD CONSTRAINT …`）。「後の版が列挙を広げて行を書き、そのあと全版を当て直す」試験を `satellite_migration_tests` に 1 本置く
- 処置: fixed D9 — 列挙の CHECK は「この版の語（'satellite' / 'forbidden'）を含む定義がまだ無いときだけ」付け替える `DO` の塊に（design D9 の SQL も同じ形へ）。試験 `satellite_migration_reapply_keeps_later_widened_checks`（使い捨ての DB で、後の版を模して列挙を広げ `future_kind` の行を書いてから全版を当て直し、広げた定義が残る）を足した。前の移行に戻すとこの試験が `13:37`（当て直しの panic）で落ちることを確かめた

## R4. 「保存は値全体の SHA-256」を固定するテストが無く、ハッシュの関数を変えても全部通る
- 成果物: crates/server/src/satellite_admin_tests.rs:483-487（`satellite_credential_issue_value_carries_identifier`）/ crates/server/src/web_session.rs:107（`sha256`）
- 根拠: ハッシュの唯一の断言は `assert_eq!(stored, web_session::sha256(&value))` で、実装の関数を実装の関数と比べている（試験の種の `seed_credential` も同じ関数で入れる）。M4（`sha256` を `Sha256::digest(format!("{s}!"))` に）で `cargo test -p ashiato-server` の `satellite_credential`（10 passed）・`satellite_authn`（3）・`satellite_access`（5）・`every_route`（1）・`web_session`（19）が全部 rc=0。関数が変わると、発行済みの資格情報とログインの印がすべて黙って引けなくなる。値は本体に無いので戻せず、衛星ごとに発行し直すことになる
- kind: technical
- 提案: 期待値を実装の外で作る。DB の `sha256(convert_to($1, 'UTF8'))` で発行の応答の値から計算した bytea を、保存された `token_sha256` と比べる（固定の値と、別の道具で出した 16 進の固定ベクタでもよい）
- 処置: fixed 5.1 — `satellite_credential_issue_value_carries_identifier` の期待値を DB の `sha256(convert_to($2, 'UTF8'))` で作り、保存された `token_sha256` と比べる（実装の `web_session::sha256` を使わない）

## R5. 「行は消せない」の試験が共有の試験の DB に TRUNCATE を撃ち、並んで走る衛星の試験を deadlock させる —— 完了の判定のテストが 3 回に 1 回落ちる
- 成果物: crates/server/src/satellite_migration_tests.rs:144-170（`satellite_migration_rows_cannot_be_deleted`。`testdb::pool()` の共有の DB に `TRUNCATE core.satellite CASCADE` / `TRUNCATE core.satellite_credential`）/ crates/server/src/satellite_routes_tests.rs（`every_route_refuses_satellite`）
- 根拠: 壊していない作業ツリーで `cargo test -p ashiato-server satellite_` を 3 回続けると、1・2 回目は 42 passed、3 回目は rc=101（`every_route_refuses_satellite` が `GET /archives/status` で `left: 500 right: 403`）。M4 の走り（作り直した DB）でも `40P01 deadlock detected`（`seed_credential` の INSERT）と 500（`satellite_authn_revoked_credential_with_cookie_is_unauthorized`）で 2 本落ちた。`docker compose logs db` で deadlock の相手を見ると `[341] STATEMENT: TRUNCATE core.satellite CASCADE`。BEFORE TRUNCATE のトリガが拒む前に、TRUNCATE が 2 表の AccessExclusiveLock を取りに行き、並んで走る判定の `UPDATE core.satellite_credential … FROM core.satellite`・種の INSERT と取り合う。final review の re-review が「範囲外の所見」として残した cold start の deadlock（相手は特定できていない、とした）はこれ。同じ節は「移行の `DROP TRIGGER`」を疑っていたが、相手は移行ではない。CI は毎回作り直した DB で全走するので、`rust` の job がこの Story の完了の判定で赤くなりうる
- kind: technical
- 提案: DELETE / TRUNCATE を拒むことの確かめは、同じファイルのほかの試験と同じく `stay_tests::fresh_db()` の使い捨ての DB に移す（共有の DB に撃つのは UPDATE だけにする）

---
- 処置: fixed 1.2 — `satellite_migration_rows_cannot_be_deleted` を `stay_tests::fresh_db()` の使い捨ての DB に移した（共有の試験の DB には DELETE / TRUNCATE を撃たない）。`cargo test -p ashiato-server satellite_` を 3 回続けて 43 passed ×3

## final review（4ee5f44..c940f70）（別の席の記録。処置は呼び出し元が付けた）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。入力は review package
`.superpowers/sdd/tasks/review-4ee5f442..c940f703.diff`、Task 1〜10 の指摘の記録の Minor / ⚠️、Task 7 の ledger の parked（F1）、`docs/handoff/ST29.md`。
判定: **With fixes**。D8（衛星の判定は最も外側で 1 回）・D6（値は応答に 1 回・保存は SHA-256 だけ）・D4 / D9（読み出しの記録は uuid だけ、CHECK と二重）・D14（全 route の試験は契約から取る）は design どおり。
申し送りの st17 C2・st33 C4（`/feelings` `/feelings/reminder` `/export*` を衛星の資格情報で断る）は `every_route_refuses_satellite` で閉じている。
parked の Task 7 F1（`export_ledger_migration_applies_twice`）は、ブランチの `export_ledger_tests.rs` の変更が main の df75d2c1 と同じ字面で、`git merge-tree` でも衝突が無い —— このブランチで追加の対応は要らない。
main との統合で衝突するのは `crates/server/src/lib.rs`（use と mod の塊）と `docs/openapi.json`（`src/bin/openapi.rs` で作り直す）だけ。

## R1. main と統合すると全 route の試験（この Story の完了の判定）が HEAD の口で落ちる
- 成果物: crates/server/src/satellite_routes_tests.rs:57-66（`contract_operations`）/ :157（本文の比較）
- 根拠: origin/main の `docs/openapi.json` の `/media/originals/{sha256}`（ST09）は `['put', 'head']`（`git show origin/main:docs/openapi.json` を python で確かめた）。`contract_operations()` は get/post/put/delete/patch 以外で `panic!("知らない method …")`。HEAD は本文が削られるので `body_string == "forbidden"` も落ちる（Task 6 F1 が一般論で指していたもの）
- kind: technical
- 提案: `head`（と `options`）を足し、path の項目の method でない key（`parameters` `summary`）は飛ばす。HEAD は状態の番号だけを見る
- 処置: fixed 6.1 — a3170df6。`head` / `options` を足し、method でない key は `continue`、`trace` だけ panic。`request()` は HEAD / OPTIONS に本文を付けない。HEAD は本文を見ない。`cargo test -p ashiato-server satellite_` → 42 passed。HEAD の分岐はこのブランチの契約に head の口が無いので、統合した木で初めて通る（PR 本文に挙げる）

## R2. 書き出しの `SATELLITE_COLUMNS` の `id` の説明が「衛星の識別子」で `identifier` と紛らわしい
- 成果物: crates/server/src/export.rs:327
- 根拠: 同じ表の `identifier` の説明が「衛星の識別子の文字列（逆ドメイン形）」。Task 7 F2
- kind: technical
- 処置: fixed 7.1 — a3170df6。「衛星の番号（uuid）」に。`export_satellite_tables_and_columns_are_classified` は緑

## Minor（merge 前に直さない。理由つきで ledger に park）

final reviewer が Minor とし、merge を止めないとしたもの。SDD の Final Review の「fix は 1 回だけ」に従い、R1・R2 の fix に入れなかった。
- `web/src/SatellitesPanel.tsx` の `revoke` が `busy` を立てない（Task 9 F3）。2 回目の DELETE も COALESCE で 204 —— 実害は小さい
- `SatellitesPanel.tsx` の確認の「やめる」が `destructiveStyle` を流用（Task 9 F4）。44 px のための流用で見た目の問題だけ
- `web_session.rs` の `session_get` が 403 から衛星と推して `decided` を読み直す（Task 3 F1）。いまの 403 は衛星だけ。衛星以外の 403 を足す Story で `decided` を先に match する形に
- `access_log.rs` の `AccessEntry.satellite` が `pub`（「`for_caller` からだけ埋まる」は DB の CHECK `access_log_satellite_ids` 頼み）
- `satellite_form_tests.rs` の Rust 側の突き合わせは識別子だけ。名前の上限 40 と Rust の定数の一致は web 側の試験（`satellites.test.tsx:187`）だけ（Task 2 F2）
- 移行の access_log の CHECK を当てるたびに DROP / ADD（D9 の指定どおり。表が大きくなると起動が遅くなる。Task 1 F4）
- `tools/check-immutable.d/202610241210_st29-satellite-credentials.sh` (b) の行の組み合わせが読み手に紛らわしい（Task 1 F3）。検査は通る
- `web/e2e/settings-export.spec.ts:144` の題が「最初の節は「全記録の書き出し」」のまま（中身は agreed-test-changes.md のとおり直してある。印の `// Scenario:` は正しい）
- `docs/handoff/ST29.md` の st28 Q4 に「ST44 へ移した」の追記が無い（中身は `docs/stories/ST44.md:14` にある）—— finish の PR 本文の「申し送り」で触れる
- `POST /satellites` で資格情報が無く本文が壊れていると 401 でなく 422（Task 4 F1。lib.rs の既存の口と同じ形）
- `tools/check-satellite-boundary.sh` のループ変数 `f` が `local` でない・self-test が `forbidden_in_deps` だけ（Task 8 F1・F2）

## fix（c940f70..a3170df）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。R1・R2 とも ADDRESSED、fix の diff に新たな Critical / Important は無し。
範囲外の所見: DB を新しく作った直後の 1 回目の `cargo test -p ashiato-server satellite_` で `satellite_routes_tests` の 2 本が `40P01 deadlock detected`
（`seed_credential` の INSERT と、別の接続が移行で `satellite_credential` に AccessExclusive を取りに来た。移行は `DROP TRIGGER IF EXISTS` を当てるたびに走らせる）。
続く 2 回は緑。同じプロセスの移行は `testdb::pool()` の OnceCell と助言ロックで直列なので、相手の接続は特定できていない —— CI の cold start で再現したら `ci_fix` で拾う。

## security review（4ee5f44..0af733c）

見た経路:
- 衛星の値の `Bearer ashsat_…` → `refresh_cookie` の `decide` → `authorize_quiet`（`lib.rs:490-509`。値全体の SHA-256 で `revoked_at IS NULL` かつ `removed_at IS NULL` の行だけ引く）→ `access_log::middleware`（`access_log.rs:163-186`）が 403 の行を書いてハンドラを呼ばずに 403。取り込み（`/ingest` 等）の本文も読まない（4）
- 同じ求めで middleware を通らない `GET /session` → `authorize` の `Satellite` は 403（`lib.rs:563-565`）→ `session_get` が 403 の行を書く（`web_session.rs:434-449`）（4: `exempt_routes_ignore_satellite`）
- 引けない `ashsat_` の値（取り消した・外した衛星・知らない値）+ 有効な cookie → 印を見ずに 401（`lib.rs:490` の早期 return）（4: `satellite_authn_revoked_credential_with_cookie_is_unauthorized`）
- 管理の口 5 本 → `web_session_only`（`satellite_admin.rs:84-90`）で `WebSession` だけを通す。端末が持つ API の合言葉は 403、無しは 401（4: `satellite_admin_only_api_token_is_forbidden` が 5 本とも 403 と一覧が変わらないことを見る・`…_without_credential_is_unauthorized`）
- 画面からの書き換え（CSRF）: cookie は `HttpOnly; Secure; SameSite=Strict`（`web_session.rs:196`）。CORS の層は無く、`POST` は `Json` 抽出（content-type が要る）・`DELETE` は preflight が要る（2）
- 発行 → 値は `OsRng` 32 バイト、保存は SHA-256 だけ（`satellite_admin.rs:368-384`）、応答は 1 回だけ（`Cache-Control: no-store` は最も外側の層。`lib.rs:3534`）。`IssuedCredential` の `Debug` は値を出さない。一覧は値もハッシュも載せない（4: `satellite_credential_issue_value_is_never_listed`）
- エラーの出口: `internal_at` は sqlstate だけをログに出す（`lib.rs:3454-3461`。一意制約違反の DETAIL＝ハッシュや識別子は出ない）。400/409 の本文は種別の固定の語だけ（2）
- サーバのログ → smoke.d が 2 台目のサーバのログに `ashsat_…_{43}` の形と発行した値そのものが無いことを grep（`tools/smoke.d/202610241500_…:50-55`）（2: 台本を読んだ。この席では smoke を走らせていない）
- 画面: 値は React の state だけ（`SatellitesPanel.tsx:28,99`）。保存領域・URL・console には出さない。名前・識別子は JSX のテキストで出す（`dangerouslySetInnerHTML` は無い）（2）
- 書き出し: `satellite` は `Include`（識別子・名前・時刻）、`satellite_credential`（ハッシュ）は `Exclude`（`export.rs:461-475`）（2）。依存の追加は無い（`Cargo.toml` / `package.json` は差分に無い）。新しい listen も外部への送り先も無い（smoke の 2 台目は `127.0.0.1`）（2）

安全の決め手: **router の全 route は `route_layer(access_log::middleware)` の下にあり、その middleware は `Caller::Satellite` をハンドラより前で 403 にする。middleware の外にある `/session` も `authorize` で 403 になる** —— 確かさ 4。
`scripts/quiet-run unit -- cargo test -p ashiato-server satellite_` → `test result: ok. 42 passed; 0 failed`。この中の `every_route_refuses_satellite` は
契約の全 operation に衛星の値を付けて呼び、全部 403・本文 `forbidden`・読み出しの記録に 1 行ずつを見る。あわせて `lib.rs` の `.route(` の path の集合が契約と一致すること、`.merge(` `.nest(` が無いことも見るので、後から足した口も外れない。
管理の口が画面のログインだけで通ることは `satellite_admin_only_api_token_is_forbidden`（同じ走りで ok）。

指摘なし

範囲外: `/selftest/panic`（`lib.rs:3627`）は `route_layer` の後で生える口で middleware の外にある（環境変数で開けたときだけ。既存で、この差分は触っていない）

## /code-review（4ee5f44..0af733c）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R6. DELETE /session (and POST /session) skip the access_log middleware and never look at the satellite judgement, so a satellite credential can still use the /session routes, and the log row records it as `web_session`/`none` instead of `satellite`/`forbidden`.
- 成果物: crates/server/src/web_session.rs:387
- 根拠: Send `Authorization: Bearer ashsat_...` together with a valid session cookie to DELETE /session. authorize would treat this caller as a satellite (D18), but session_delete only checks session_valid(cookie), so it logs the session out with 204 and writes credential='web_session', outcome='logout'. That breaks D8 ('a satellite credential passes no route yet'), and the log never shows that a satellite was the caller. session_get got its own 403 branch for this case; session_delete and session_post did not.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: spec（satellite-platform「衛星の資格情報では、まだどの口も通らない」）は、ログイン・ログアウトを資格情報の要らない口とし「衛星の資格情報を付けた求めと付けない求めとで同じ応答を返す」と定める（Scenario `資格情報の要らない口は衛星の資格情報の有無で応答が変わらない`・`exempt_routes_ignore_satellite`）。`DELETE /session` が印だけを見て 204 を返すのは、その定めどおり。403 にすると spec に反する。`GET /session` は資格情報の要らない口に入らないので 403（別扱いは意図どおり）

## R7. migrate() reapplies every migration on each run, and this one does DROP CONSTRAINT / ADD CONSTRAINT for three CHECKs on core.access_log every time. That validates the whole append-only table under an ACCESS EXCLUSIVE lock on every run.
- 成果物: migrations/202610241200_satellite_credentials.sql:1231
- 根拠: access_log gains one row per request and rows are never deleted, so after months of use every migrate() scans the full table three times while holding an exclusive lock. If migrate runs while a server is up, access_log writes stop and all requests wait (or fail with 500). Wrapping this in a DO block that checks pg_constraint, or using NOT VALID followed by VALIDATE only once, avoids it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D9 — R3 と同じ直し。当て直しでは `pg_constraint` を見るだけで DROP / ADD しない（全走査も ACCESS EXCLUSIVE も取らない）

## R8. The access_log_satellite_ids CHECK only constrains the case where both IDs are set. A non-satellite row with only one of the IDs set passes.
- 成果物: migrations/202610241200_satellite_credentials.sql:1239
- 根拠: INSERT credential='api_token', satellite_id=<uuid>, satellite_credential_id=NULL. Left side: false = (true AND false) = false, so the CHECK passes. The intended rule ('IDs only on satellite rows', which AccessEntry's doc comment says matches the DB CHECK) is not enforced, and half-filled rows can stay in the append-only table forever. It should be `CASE WHEN credential='satellite' THEN both NOT NULL ELSE both NULL END`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D9 — 制約を `access_log_satellite_pair`（`CASE WHEN credential = 'satellite' THEN 2 つとも NOT NULL ELSE 2 つとも NULL END`）に替え、前の `access_log_satellite_ids` は外す（戻し手順も両方を外す）。`satellite_migration_constraints_hold` に「衛星でない行に片方だけ」の 2 例を足し、前の移行ではその断言（122 行）で落ちることを確かめた

## R9. The lock only blocks DELETE/TRUNCATE. UPDATE is left fully open, so revoked_at can be reset to NULL (un-revoke), and identifier, token_sha256 and builtin can be rewritten.
- 成果物: migrations/202610241200_satellite_credentials.sql:1200
- 根拠: Someone with the app role (which has UPDATE through grants.sql) runs `UPDATE core.satellite_credential SET revoked_at = NULL` and a revoked credential works again. `UPDATE core.satellite SET builtin=false` makes a built-in satellite removable. Changing identifier contradicts the UI text 'cannot be changed later'. Revoke and remove are supposed to be one-way, but the DB does not hold that.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D7 が UPDATE を許すと決めている（外す・取り消す・最後に使った時刻・名前・登録し直しの `removed_at = NULL`）。挙がった筋書きは app の役割で任意の SQL を撃てる者だが、その者は UPDATE を止めても `INSERT INTO core.satellite_credential` で自分の知る値のハッシュを足せるので、取り消しの一方向を DB で守っても守れるものが増えない。識別子・`builtin`・ハッシュを書き換える口は無い（口の試験が固定する）

## R10. IDENTIFIER_PATTERN is described as the canonical source, but check_identifier is a separate hand-written matcher that does not use it. The tests only compare the pattern string with the migration, so the Rust check itself can drift from the canonical pattern.
- 成果物: crates/server/src/satellite.rs:349
- 根拠: Someone loosens the pattern later (for example to allow `_`) and changes the constant and the migration together. satellite_form_tests stays green, matches_identifier_shape keeps the old rule, and the API rejects identifiers the DB would accept. Either generate the check from the regex, or add a test that compares the regex and the matcher over the same set of inputs.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `satellite_form_tests::identifier_check_agrees_with_migration_regex_in_db` が、移行の字面から取り出した正規表現を PostgreSQL の `~` にかけ、Rust の `check_identifier` と 18 件で突き合わせている（`dev_example.booklog` を含む）。筋書きの「`_` を許すよう定数と移行を一緒に緩める」と、その 1 件で DB と Rust が食い違って落ちる

## R11. The satellite credential check does an UPDATE (writing last_used_at) on every request, even though every satellite request currently ends in 403. That puts a DB write and a row lock on the hot path.
- 成果物: crates/server/src/lib.rs:491
- 根拠: A satellite polls in a misconfigured loop. Every request costs an UPDATE on the same credential row plus WAL, and concurrent requests serialize on the row lock. last_used_at only needs minute-level precision, so `WHERE last_used_at IS NULL OR last_used_at < $2 - interval '1 minute'`, or a SELECT followed by a throttled UPDATE, would be enough.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md:135` は「衛星の資格情報を付けた求めが届く THE SYSTEM SHALL 応答の結果にかかわらず、その資格情報の最後に使われた時刻をその求めの時刻にする」と定める。1 分に丸めるとこの定めに反する。重さは `openspec/changes/st29-satellite-credentials/design.md:253` で受けてある（衛星はまだ何も読めず頻度は低い。ST45 で読み始めて重くなれば丸める）

## R12. The cancel button in the remove-confirmation dialog uses destructiveStyle, so both buttons ('remove' and 'cancel') look destructive. This looks like a copy-paste slip.
- 成果物: web/src/SatellitesPanel.tsx:270
- 根拠: On the remove confirmation the user cannot tell from the look which button is the safe one. The other cancel buttons (issue and register forms) use controlStyle, so this one is inconsistent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `destructiveStyle` は `controlStyle` と同じ色で、違いは大きさ（44 px。NFR-20）と余白だけ（`web/src/surface.ts:86-88`）。確認の「外す」と「やめる」は見た目の色で区別されるものではなく、どちらも 44 px であることを `satellites.test.tsx` の「操作の対象に大きさの指定が付いている（… 確認は 44 px）」が固定している。`controlStyle` に替えるとこの試験が 24 px で落ちる（実測）

## R13. revoke does not set busy and has no confirmation, so double-clicks and other actions can run at the same time. A credential is revoked on a single press.
- 成果物: web/src/SatellitesPanel.tsx:113
- 根拠: Press 'revoke' and immediately press 'issue' or another credential's 'revoke'. busy stays false, so both requests and two reloads run at once, and an older list can overwrite a newer one. A wrong press cannot be undone: the satellite that was using the value gets 401 from the next request on.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 — `revoke` で `busy` を立て、終わったら下ろす（他の操作と重ならない）。確認をはさむ件は rejected: design D17（仮）が「取り消しは確認をはさまず」と決め、反転条件（押し間違いの違和感が出たとき）も持つ

## R14. name_owner and NAME_PART_MAX_LEN are not used anywhere in production code, only in tests, so they are dead code for now.
- 成果物: crates/server/src/satellite.rs:391
- 根拠: Shipping an unused public API means that when a later story starts using it, the code's behaviour is not tied to any real caller, and the spec and implementation can drift apart. Either cut it now or move it into the story that uses it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `name_owner` と `NAME_PART_MAX_LEN` は tasks 2.1 と Scenario `名前の持ち主は区切りより前の識別子で決まる`（design D5 / D15（仮））が求める純粋関数で、試験 `name_owner_is_the_identifier_before_the_separator` がその値の表を固定している。使い手（FR-86 の名前の持ち主）は後の Story

## R15. satellites_get runs a full scan of every credential for each satellite (O(satellites x credentials)) and clones each label.
- 成果物: crates/server/src/satellite_admin.rs:669
- 根拠: With many satellites and credentials, every list request repeats the work. Grouping into a HashMap<Uuid, Vec<CredentialOut>> first, or nesting with json_agg in a single query, makes it linear.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 — 資格情報を衛星ごとに 1 回で `HashMap` へ振り分け（発行の順は保つ）、label は clone せずに移す。`satellite_register` 10 本ほか `satellite_` 43 passed

## finish の fix（0af733c..bed15ab）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。fixed の R3・R4・R5・R7・R8・R13・R15 は ADDRESSED、rejected の R6・R9・R10・R11・R12・R14 は理由に同意。fix の diff に新たな Critical / Important は無し。
Minor（merge 前に直さない。SDD の「fix は 1 回だけ」で park）:
- 移行の `DO` の塊は毎回 `DROP CONSTRAINT IF EXISTS access_log_satellite_ids` を撃ち、制約が無くても一瞬 ACCESS EXCLUSIVE を取る（全走査はしない。既存の `ADD COLUMN IF NOT EXISTS` と同じ性質）
- 列挙の付け替えの判定は定義の文字列の部分一致（`LIKE '%''satellite''%'`）。後の版が語を含む別の定義へ作り替えても触らない（意図どおり）
- `SatellitesPanel.tsx` の `revoke` は読み直しの後に `busy` を下ろし、`run()` は前に下ろす（不揃いだが実害なし）
- `satellite_migration_rows_cannot_be_deleted` は移行が入れる最初の衛星 `ashiato.ai-local` に依存する
