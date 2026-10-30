# ST50 code review

## final review（87a0ab0..c4ffe79）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 1 / Important 4 / Minor 11。
reviewer が手元で走らせた結果: `cargo test -p ashiato-collector-windows --lib` 311 passed、`cargo test -p ashiato-server` 1587 passed / 1 failed
（`archive_scan_interval_change_takes_effect_without_restart`。単独で ok・ST50 は触っていない。時間に依る不安定な試験として R にしない）、
`check-openapi.sh` / `check-contract-version.sh` / `check-migrations.sh` OK。
Task ごとの ⚠️ のうち、Task 9（after_send / ContractToaster の配線）・Task 10（LocationService の 404・読めない答え）・Task 5（例の書き換え禁止は検査 4 が担う。D8）は解消済みと判定。

rebase で base は de2b737b に移った（指摘の SHA は rebase 前）。fix wave は 2 つのセッションにまたがった（9699cbfc〜3cba4be3 の後で途切れ、fc957c9f〜47a9a2a9 で残りを直した）。scoped re-review（246746d2..47a9a2a9）: 16 件すべて ADDRESSED・新しい Critical / Important なし。Minor 1 件（携帯の `nextBatch` は A が下がると probe を手放しうる。D9 は版が下がらない前提）は park。

## R1. origin/main に入った ST49（`202610251000_record_component_version`）・ST23（`record_purge`）とぶつかる。rebase の後は移行の手順 3 が本物の DB で RAISE で落ち、ST49 の検査が黙って消える
- 成果物: migrations/202610300030_contract_version.sql
- 根拠: `git merge-tree` で 20 ファイルが衝突（`event_end_time.sql`・`ORDER`・`ingest.rs`・`lib.rs`・`drops.rs`・`media_missing.rs`・`export.rs`・`openapi.json`・`Outbox.kt`・`Sender.kt`・`IngestRequest.kt`・`contract.rs` ほか）/ 手順 3 の `copied_from` は ST41 の md5、門と前の版の錠の写しは ST41 の本体（ST49 の `collector_version` / `web_version` / `server_version` の検査を含まない）/ `event_version_live` に ST49 の列が無い / `.down.sql` は ST41 へ戻す / ST49 の守りは `copied_from = ARRAY['048663…']` だけを許し、ST49 の `CREATE OR REPLACE VIEW event_version_live` は `contract_version` を持たない / Task 4 の parked の Ruling が「ST49 の移行にも同じ形が要る」と既に書いている
- kind: technical
- 処置: fixed 4.1 （e1f63ed2・9699cbfc。ST49 の本体から写し直し、手順 3 の copied_from を ST49 の md5 に。ST41・ST49 の許す md5 に ST50 を足した）

## R2. `unknown_field` / `contract_unsupported` で断られている間、新しい記録が受け付けられるたびに待ちを抜けて通知が消え、次の契機で出直す（PC・携帯）
- 成果物: crates/collector-windows/src/contract_gate.rs
- 根拠: `contract_gate.rs` の `observe` の Version の分岐 / `ContractGate.kt:observe`（`rejected` が偽になると飛ばす印と `notified_at` が捨てられる）/ deep.md Q5・Q6（止まった時に 1 回・1 日 1 回）/ D8（配った印が無い間は版を上げずに欄を足せる）/ Task 8 F3
- kind: technical
- 処置: fixed 8.1 （PC 3cba4be3 `st50_sender_version_rejected_rows_keep_waiting_while_new_rows_pass`・携帯 10.1 fc957c9f / 18ddb739。D9・D14）

## R3. 携帯で再起動の直後に 24 時間未満の読めない 200 を得ると `known = true` になり、出ていた通知と画面の行が消える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: `ContractGate.kt` の Unreadable・NoEndpoint の分岐が `known = true` / `ContractNotifier.update` が通知と `store.notice`・`notifiedAtMs` を消す / Scenario「再起動しても 24 時間経つまで通知は出直さない」/ PC は `reasons_known` を Version の分岐でだけ立てる / 今の試験は「確かめる前」だけを見る
- kind: technical
- 処置: fixed 10.1 （fc957c9f。`known` は読める答えだけ、通知と画面は `decided` で決める。試験「通知を出した後の再起動の直後に読めない 200 を得ても…」）

## R4. `tools/check-openapi.sh` の `contract_version` の除外が残り、収集アプリの欄 ⊆ サーバの欄を機械で見る検査が無い
- 成果物: tools/check-openapi.sh
- 根拠: ingest・heartbeat・drops の 3 か所の除外（期限「携帯が送るようになるまで」は Task 10 で過ぎた）/ DropReport・Heartbeat の試験の `- contractVersionField` / `check-contract-version.sh` は「サーバの部分と openapi」「写しと印」しか比べない / Task 3 の ⚠️
- kind: technical
- 処置: fixed 7.2 （ceedff01。check-openapi.sh の除外 3 か所を外し、check-contract-version.sh に検査 6「収集アプリの欄 ⊆ S-01 の欄」と self-test）

## R5. 最終の HEAD での全体の検証の証跡が無い
- 成果物: openspec/changes/st50-contract-version/evidence.jsonl
- 根拠: evidence.jsonl は絞り込んだ実行だけで head は 23 種（最終の c4ffe79c を含まない）/ Task 4 で「全体の 90 本が落ちる」とされた後のサーバ全体の記録が無い / 携帯の `testDebugUnitTest` 全体・detekt・`check-immutable.sh` は未実行 / R1 の rebase で全 SHA が変わる
- kind: technical
- 処置: fixed 12.2 （edc69fd6・dec0b8e0 で全体の検証が見つけた clippy / detekt を直し、verify-run 1.1〜12.2 を 18ddb739 で PASS・47a9a2a9 で evidence に記録。server・collector-windows・Android 全体・detekt・check-*・smoke も PASS）

## R6. `ContractAnswer.contract_version` が openapi で `"minimum": 0`
- 成果物: crates/server/src/contract_version.rs
- 根拠: docs/openapi.json の `ContractAnswer` / 最初の印で口の形が凍結される（D8）/ Task 1 F1
- kind: technical
- 処置: fixed 1.1 （249268a8。`minimum = 1`）

## R7. `ContractGate.kt` の `refused` が短絡評価で `originalsRefused.getAndSet(false)` を消費しない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: `(posted && …refused) || originalsRefused.getAndSet(false)` / Task 10 F4（`OriginalUploader.lastContract` が早く返った回に更新されない）
- kind: technical
- 処置: fixed 10.1 （fc957c9f。`originalsRefused.getAndSet(false)` を先に評価）

## R8. 4 つの JSON の口は `Json` の抽出子が見出しの検査より先に本文を解釈する
- 成果物: crates/server/src/lib.rs
- 根拠: Task 2 の ⚠️（diff:469・504・535・764）/ 壊れた本文・大きすぎる本文・content-type 無しでは 426 でなく 400/413/415
- kind: technical
- 処置: fixed 2.1 （25a5cad8。本文を `Result<Json, JsonRejection>` で受け、資格情報と見出しの後に断る。`st50_header_426_before_body_rejection`）

## R9. 携帯は待っている間、契機ごとに未送信を全部復号して走査する（最大 3 回）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt
- 根拠: `Sender.flush`・`SegmentStore.hasVersionAbove`（新しい版の行は末尾にあり毎回ほぼ最後まで読む）
- kind: technical
- 処置: fixed 10.1 （76744589 で飛ばす印が無いときの読み直しを省いた。契機ごとの全走査 1 回は park —— `.superpowers/sdd/tasks/progress.md` の Ruling）

## R10. PC の通知の日数が積んだ時刻でなく `event_time` の最小から出る
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs` の `oldest`（ブラウザの履歴の古い訪問で日数が大きくなりうる）
- kind: technical
- 処置: fixed D5 （3cba4be3。置き場の控えに積んだ時刻を持つ。`st50_outbox_keeps_enqueued_time_outside_the_row`）

## R11. `ItemGate::new` が要求ごとに utoipa の schema を組み直し、`known_fields` の `unreachable!` が要求の経路にある
- 成果物: crates/server/src/contract_version.rs
- 根拠: Task 3 F2・F6
- kind: technical
- 処置: fixed 3.1 （249268a8。口ごとの static で 1 回だけ組み、router を組む時に組ませる）

## R12. `i32::try_from(v).ok().unwrap_or(1)` が i32 に収まらない版を黙って版 1 にする。移行の頭の注記が台帳の実装と合わない
- 成果物: crates/server/src/lib.rs
- 根拠: Task 4 F2・F3 / 移行の注記「`migrate()` は起動のたびに全版を当てる」（記録済みの版は飛ばす）
- kind: technical
- 処置: fixed 4.2 （249268a8 `st50_item_version_out_of_column_range_is_refused_not_v1`、注記は e1f63ed2）

## R13. `WaitStore::load` が壊れたファイルをログ無しで空として読む
- 成果物: crates/collector-windows/src/contract_gate.rs
- 根拠: Task 8 F6
- kind: technical
- 処置: fixed 8.1 （3cba4be3 `st50_gate_broken_wait_file_is_logged_and_read_as_empty`）

## R14. `contract_notice_skipped` が契機ごとにログに出続ける（PC・携帯）
- 成果物: crates/collector-windows/src/contract_notice.rs
- 根拠: Task 9 F5
- kind: technical
- 処置: fixed 9.1 （cb6986a6 `st50_notice_skipped_logged_once_per_failure`・携帯 11.1 も同じ commit）

## R15. `docs/testing.md` の「16 本」の括弧の一覧に contract-version ×2 が無い
- 成果物: docs/testing.md
- 根拠: Task 7 F2
- kind: technical
- 処置: fixed 7.2 （a7684b0b。rebase 後の「検査 21 本」を 23 本にし一覧に足した）

## R16. smoke が `/tmp/st50-426.$$` を使い、curl が落ちると残る
- 成果物: tools/smoke.d/202610301200_st50-contract-version.sh
- 根拠: Task 12 F1 / HARNESS_TMP_LEAK
- kind: technical
- 処置: fixed 12.1 （a7684b0b。`curl -w` で本文と状態符号を 1 回で受け、一時ファイルを作らない）

## code_verify（02b7fd17。2026-10-30）

### 申告と実測

申告: tasks.md 18/18 `[x]`（1.1〜12.2。7.1 の検証は 7.2 に入れてある）。検証コマンドは各 Task の本文（`CT` / `CW` / `GT` と台本）。evidence.jsonl の最後の記録は 18ddb739（HEAD 02b7fd17 との差分は docs と evidence と review だけ）。
この席では HEAD で走らせ直した（`tools/verify-env.sh` でこの worktree の DB を起こし、`.env` を読ませて直接）。

| 項目 | 検証コマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `CT st50_contract_`（4）/ `CT access_log`（9）/ `check-openapi.sh` | 一致（`--list` 6 / 13・`check-openapi.sh` rc=0「API の契約 OK」） |
| 2.1 | `CT st50_header_`（8） | 一致（9） |
| 3.1 | `CT st50_unknown_field_`（6）/ `CT st50_item_version_`（1）/ `check-openapi.sh` | 一致（7 / 3・rc=0） |
| 3.2 | `docs/collector-contract.md` の grep 4 つ | 一致（rc=0） |
| 4.1 | `check-migrations.sh` / `cargo build -p ashiato-server` | 一致（rc=0 / rc=0） |
| 4.2 | `CT st50_record_version_`（8）/ `CT hash_is_pinned`（1） | 一致（8 / 2） |
| 4.3 | `CT st50_migration_`（2）/ `CT st50_lock_`（2）/ `check-immutable.sh` | 一致（2 / 2・rc=0「書き換え禁止 OK」。ST50 の節「埋め戻しで外した 2 つの錠が張られている」を通る） |
| 5.1 | `CT st50_released_`（3） | 一致（3） |
| 6.1 | `CT st50_export_`（2）/ `CT export_`（100） | 一致（2 / 188） |
| 7.1 / 7.2 | `check-contract-version.sh --self-test` / 本体 / `GT St50EnvelopeShapeTest`（1）/ `CW st50_envelope_shape`（2） | 一致（rc=0「self-test OK」/ rc=0「送信契約の版 OK（版 1）」/ 3 / 2）。ci.yml:99-100 は `check-released-migrations.sh` の 2 行（95-96）の後に置かれている |
| 8.1 | `CW st50_sender_`（9）/ `CW sender::`（20） | 一致（10 / 21） |
| 9.1 | `CW st50_notice_`（10）/ clippy `-D warnings` | 一致（12・clippy rc=0） |
| 9.2 | grep 1 本 / **終了条件は CI の `collector-windows-runtime` が緑** | **不一致（終了条件の証跡なし）**。grep は rc=0。PR も push も無い（`gh pr list --head feat/st50-contract-version` → `[]`、`git ls-remote origin` に枝が無い）ので、本物の Windows の job は一度も走っていない → R20 |
| 10.1 | `GT St50SenderTest`（13）/ `GT OriginalUploaderTest`（30）/ `SenderTest`（14） | 一致（21 / 30 / 14） |
| 11.1 | `GT St50ContractNoticeTest`（12）/ `RetentionNotifierTest`（10）/ `MainActivityTest`（13）/ detekt | 一致（16 / 10 / 13・detekt rc=0） |
| 12.1 | `tools/smoke.sh` | 一致（最終行「縦串 OK」。ST50 の節 `OK /contract`・`OK 426`・`OK 今の形`・`OK unknown_field`） |
| 12.2 | `check_scenarios.py` / `provisional.py check` | 一致（scenarios: OK / rc=0。`[wait]` 1 件は既存の device-collection の Scenario で ST50 の 80 本に無い） |
| 全体 | `cargo test -p ashiato-server` / `cargo test -p ashiato-collector-windows --lib` / `:app:testDebugUnitTest` | 1750 passed 3 ignored・server_startup 25 passed / 320 passed / 742 本 failures 0 |

### 手 1: 固定値を独立に再計算する

- 移行の md5 の 6 つ（ST50 の `copied_from` と `this_version`、ST41・ST49 に足した ST50 の本体の値）を、移行ファイルの `$fn$…$fn$` の中身から python の `hashlib.md5` で計算し直した。**全部一致**（ST50: `require_version_or_ledger` 67356b71…・`reject_version_rewrite` f931e414…、ST49: e3651d34…・f2caef81…、ST41: 048663f9…・29764325…）
- 内容の鍵の固定値は変わっていない。`git diff de2b737b..HEAD` で `hash_is_pinned` に触るのは `heartbeat_hash_is_pinned` の構造体に `contract_version: None` を足した 1 行だけで、期待値は元のまま（Global Constraints「`hash_is_pinned` の固定値を変えない」を満たす）

### 手 2: ガードをわざと壊す

壊すたびに戻し、前後で `git status --porcelain` と `git diff` が空のままであることを確かめた。

| 壊したもの | 実行 | 結果 |
|---|---|---|
| ST50 の移行の 2 つの錠（migrations/202610300030_contract_version.sql:112・186）から `contract_version` を外す | `CT st50_lock_` | 落ちた（2 failed。contract_version_tests.rs:1247・1302） |
| S-01 の `CONTRACT_VERSION` を 2 に | `./tools/check-contract-version.sh` | 落ちた（rc=1「S-01の CONTRACT_VERSION（2）が写しの version（1）と違う」） |
| PC の送る契機（runtime.rs:813-817）で、答えに依らず全部を `u32::MAX` で送る | `cargo test -p ashiato-collector-windows --lib` | 落ちた（`st50_runtime_notifies_once_when_waiting_for_server_update` が runtime.rs:1418） |
| PC の送る契機（runtime.rs:816）で、口が無い・読めない答えを版 1 として送る（`_ => None` → `_ => Some(1)`） | `cargo test -p ashiato-collector-windows --lib` | **落ちなかった**（320 passed）→ R17 |
| 携帯の送る契機（LocationService.kt:981）から確かめを外す（`contract = null`） | `:app:testDebugUnitTest` 全部 | **落ちなかった**（`compileDebugKotlin` と `testDebugUnitTest` を実行して 742 本 failures 0）→ R17 |

検査の外側: `check-contract-version.sh` の本体は、配った印（`v*`）が無い間は印との比較をしない（「配った印が 0 個…印との比較はしない」）。D8 どおりで、比較の側は `--self-test` が使い捨ての木で印を打って見ている。

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . st50-contract-version` rc=0（scenarios: OK）。
主張の階層とテストの階層が食い違うのは次の 2 つ:

- device-collection / desktop-collection の「問い合わせの口が無い S-01 には（PC も）何も送らない」「読めない答えの S-01 には何も送らない」: spec の主語は収集アプリ（送る契機の頭で 1 回確かめる）。試験は送り手（`Sender.flush(answer)`・`Sender::flush(None)`）に答えを手で渡す層だけで、本物の契機（PC `Runtime::send_at`・携帯 `Drainer` を組む `LocationService`）が答えを送り手に渡すことは見ていない。`LocationServiceTest` の `contractReply` は既定の「今の版」から一度も変えられていない（R17）
- desktop-collection「更新を促す通知は途切れの通知を置き換えない」: Linux の試験は偽の `Toaster` が受けた札の文字列を比べる。OS の通知の並びは 9.2 の Windows の試験だけが見るが、まだ走っていない（R20）

軽いもの: `St50SenderTest.kt:394` に「止まっている間は 1 日に 1 回だけ通知が出直す」の印があるが、その試験（:396〜）は通知を見ていない。同じ印は `St50ContractNoticeTest.kt:245` にもあり、そちらが観測しているので担保は欠けていない（紛れた印）。

### 手 4: 本人の決定が test で固定されているか

| 決定 | 定数・値 | 書き換え | 結果 |
|---|---|---|---|
| Q10・D22（読めない 200 が 24 時間続いたら促す）PC | contract_gate.rs:17 `UNREADABLE_LIMIT_HOURS = 24` | 48 | 落ちた（5 本） |
| Q5（1 日 1 回）PC | contract_notice.rs:17 `RENOTIFY_HOURS = 24` | 12 / 36 | 落ちた（1 本 / 3 本） |
| Q10 携帯 | ContractGate.kt:72 `UNREADABLE_LIMIT_HOURS = 24L` | 48L | 落ちた（St50SenderTest 1・St50ContractNoticeTest 3） |
| Q6（1 日 1 回）携帯 | ContractNotifier.kt:112 `REPEAT = 24h` | 12h | 落ちた（St50ContractNoticeTest 1） |
| D8（版は 1） | contract_version.rs:19 `CONTRACT_VERSION = 1` | 2 | 落ちた（check-contract-version.sh rc=1） |
| Q1（作った時の版を 1 行ごとに焼き込む） | — | — | `st50_sender_burned_version_kept`（PC。積んだ後に送る版を差し替えても行の版は 1）・`St50SenderTest.kt:141`（携帯。定数を 2 に変えた後も 1 を載せる）が固定 |
| Q7（欄の追加・削除・型・必須を落とす） | — | — | `check-contract-version.sh --self-test`（rc=0）の「収集アプリだけが欄を足して…」「既存の欄の型や必須を変えて…」「欄を消して…」が落ちる側を見る |
| Q8（ベータ版の印も配った印） | — | — | 同じ `--self-test` の「ベータ版の印も配った印として比べる」（印は `v0.1.0-beta.1` だけ） |
| Q9（記録ごとに止める） | — | — | `st50_sender_sends_accepted_versions_only`（PC）・`St50SenderTest.kt:92`（携帯。版 1 の 3 件だけを送り、版 2 の 2 件は残る）が固定 |
| Q2・Q4（`unknown_field`・426・見出しの名前） | — | — | 試験と smoke が文字列と状態符号で固定（`wire.json` も同じ値を持つ） |
| Q3（6 つの口すべて） | — | — | S-01 は口ごとの試験（`st50_header_other_mouths_426`・`_original_426`・`_original_head_426`）。収集アプリの側は配線が固定されていない（R17） |

### 手 5: tasks の `[x]` と実体

上の表のとおり、18 項目の検証コマンドは全部実在して rc=0、本数の下限も満たす（`cargo test --test` の型は使っていない）。
足りないのは 9.2 の終了条件（本物の Windows の CI が緑）だけ（R20）。

### 手 6: 隙間

- 時計が先へ進んだ状態で最初の読めない 200 を得ると、時計が戻った後は数え始めが未来に残り、何日経っても（e）に入らない。携帯は 90 日で行を捨てるので、Q10 の「促されないまま捨てる」が起きる（R18。PC・携帯とも実測）
- PC の生存信号の溜めは、新しい版の収集アプリが書いた行の知らない欄を、書き直しのときに消す。design の「収集アプリを古い版へ戻す」の行が前提にしている ST49 D3（仮）は PC の生存信号に無い（R19。実測）
- PC の `version_rejected`（断られた行の印）はメモリだけだが、再起動の後は同じ行が送り直されて同じ断りで立ち直る。行は消えない（指摘にしない）
- 携帯の `SegmentStore` は行を書き直さない（追記と `.acked`）。送る時は `withUnknownFields` で行だけが持つ欄を足す。知らない欄が落ちる経路は見つからなかった
- 当て済みの ST41・ST49 の移行の書き換えで指紋の警告が出ることは、ST49 の移行の冒頭（202610251000_record_component_version.sql:4-8）に書いてある。ST49 の R14 と同じ扱い（指摘にしない）

### 手 7

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st50-contract-version` → `[]`）。

### finish の処置と scoped re-review（b65703b3）

R17〜R29 の 13 件: fixed 8（R17・R19・R21・R22・R25・R28・R29、R18 は D25（仮））・rejected 5（R20・R23・R24・R26・R27）。
scoped re-review（独立の reviewer、b65703b3 だけ）: 直した 8 件すべて ADDRESSED・新しい Critical / Important なし。
Minor 2 件のうち「PC の R17 の試験に読めない 200 の Scenario の印が無い」は、desktop-collection にその Scenario が無いので該当なし。
「PC の `send_held` は 401 と到達できないを見分けない」は park（`.superpowers/sdd/tasks/progress.md` の Ruling）。

## R17. 送る契機の頭の確かめを外しても（PC は口が無い・読めない答えを版 1 として送っても）、どの試験も落ちない
- 成果物: crates/collector-windows/src/runtime.rs / collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceTest.kt
- 根拠: runtime.rs:816 の `_ => None` を `_ => Some(1)` に変える → `cargo test -p ashiato-collector-windows --lib` 320 passed（rc=0）/ LocationService.kt:981 の `contract = ContractGuard({ contractGet() }, contractGate)` を `contract = null` に変える → `:app:testDebugUnitTest` 742 本 failures 0（`compileDebugKotlin`・`testDebugUnitTest` は実行済み）。`contract = null` のとき `Drainer.tick` の答えは null で、`Sender.flush(null)` は「確かめない呼び方」で全部を送り（Sender.kt:245-249）、`OriginalUploader.run(null)` は `Int.MAX_VALUE` まで送り（OriginalUploader.kt:132-133）、`onContract` も呼ばれない —— 携帯の ST50 の振る舞いが全部消えても緑 / 「何も送らない」の Scenario の担保は送り手に答えを手で渡す層だけ（`St50SenderTest` の `FiveSenders.flushAll`・`st50_sender_no_contract_endpoint`）/ `LocationServiceTest.kt:162` の `contractReply` を変える試験が無い
- kind: technical
- 提案: 携帯は `TestableLocationService` で `contractReply = Responded(404, "")` と読めない 200 にし、`drainer.tick()` 相当の 1 回で記録・生存信号の偽の送り先に 1 件も届かないことを見る。PC は `runtime::tests` で `get` が 404 と読めない 200 を返す Transport を渡し、`post` が 0 回であることを見る
- 処置: fixed 10.1 （b65703b3。PC `st50_runtime_send_holds_everything_without_a_readable_answer`（404・読めない 200 で `post` 0 回）・携帯 `LocationServiceTest`「本番の配線は 404 と読めない 200 の答えでどの口にも送らず行を残す」。runtime.rs の答えを版 1 に変える・`contract = null` にすると、それぞれこの試験が落ちることを確かめて戻した）

## R18. 時計が戻ると、読めない 200 の数え始めが未来に残り、何日経っても「サーバを更新してください」に入らない（携帯は 90 日で捨てる）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt / crates/collector-windows/src/contract_gate.rs
- 根拠: 数え始めは壁時計のまま持ち（ContractGate.kt:198-199 `store.unreadableSinceMs ?: now`・`now - since >= UNREADABLE_LIMIT`、contract_gate.rs:192-194 `get_or_insert(now)`・`now - since >= 24h`）、数え直すのは読める答えだけ / 一時的な試験で確かめた（戻し済み）: 2026-10-30 を基に、+365 日で最初の読めない答え → その後 +0h・+24h・+48h・+30 日・+89 日で読めない答え → 携帯（Robolectric、`ContractGate(ContractWaitStore(prefs))`）・PC（`ContractGate::new`）とも全部 `waiting=false` / 携帯の未送信には 90 日の上限が掛かる（FR-125 の★・NFR-7）ので、Q10 で本人が否とした「促されないまま 90 日で捨てる」がこの経路で残る / 保持の側は壁時計の戻りに備えて `AgeClock` を使っている（ContractNotifier.kt:32 も日数に `age()` を使う）
- kind: daily （irreversible から直した。直した側は何も捨てない —— 数え始めを今より前にしか動かさず、Q10 の「24 時間続いたら促す」に沿う。選び方（今へ置き直す / AgeClock で数える）だけが仮）
- 提案: 数え始めが今より後なら今へ置き直す（読める答え以外で数え直すのはこの 1 つだけ、と注記）。携帯は `AgeClock` で数える形も候補。試験は「未来の数え始め → 24 時間後に（e）」を PC・携帯に 1 本ずつ
- 処置: fixed D25 仮 （b65703b3。PC `st50_gate_unreadable_count_from_the_future_restarts_at_now`・携帯 St50SenderTest「読めない 200 の数え始めが未来に残っていれば今へ置き直し…」。反転条件は design D25（仮））

## R19. PC の生存信号の溜めは、新しい版の収集アプリが書いた行の知らない欄を、書き直しのときに黙って消す
- 成果物: crates/collector-windows/src/contract.rs / crates/collector-windows/src/outbox.rs / openspec/changes/st50-contract-version/design.md
- 根拠: `HeartbeatRequest`（contract.rs:380-400）は `#[serde(flatten)] unknown` を持たない（`IngestRequest` は contract.rs:335-337 で持つ）/ 一時的な試験で確かめた（戻し済み）: `contract_version: 2` と `"future":"KEEP"` を持つ行と版 1 の行を置いた `Outbox<HeartbeatRequest>` を開き、版 1 の行を `remove` → 書き直したファイルにも送る本文にも `future` が無く、`contract_version` は 2 のまま / design.md:256 は「収集アプリを古い版へ戻すと、溜めに残った新しい版の行（ST49 D3（仮）で欄ごと保たれた行）が来る」と書くが、PC の生存信号はこの前提に入らない。欄の落ちた行は版 2 を名乗ったまま版 2 の S-01 に届くので、知らない欄の検査（D2）でも版の検査（D4）でも止まらない
- kind: technical
- 提案: `HeartbeatRequest` にも `#[serde(flatten)] unknown` を足し、`IngestRequest` と同じ試験（読み戻した知らない欄を送り返す）を足す。足さないなら design の 256 行目に「PC の生存信号は戻すと新しい欄を失う」を代償として書く
- 処置: fixed 8.1 （b65703b3。`HeartbeatRequest` に `#[serde(flatten)] unknown`、`st50_heartbeat_unknown_field_survives_rewrite`。design の 256 行目の前提はそのまま成り立つ）

## R20. 9.2 は `[x]` だが、終了条件の「本物の Windows の CI が緑」が一度も満たされていない
- 成果物: crates/collector-windows/tests/runtime_windows.rs / openspec/changes/st50-contract-version/tasks.md
- 根拠: tasks.md:190-191「grep は置いたことだけを見る。終了条件は CI の `collector-windows-runtime` job が緑であること」/ `gh pr list --head feat/st50-contract-version` → `[]`、`git ls-remote origin refs/heads/feat/st50-contract-version` → 空。`st50_contract_toast_coexists_with_outage` は一度も走っていない / desktop-collection「更新を促す通知は途切れの通知を置き換えない」の OS の層はこの試験だけが見る（Linux の `st50_notice_separate_tag` は偽の `Toaster` の札の文字列）
- kind: premise
- 提案: PR の CI で `collector-windows-runtime` が緑になったのを確かめてから 9.2 を閉じたと扱う（merge gate が CI を待つなら、そこで足りる）。走る前に `[x]` を付けない
- 処置: rejected: 終了条件は merge gate が機械で見る。`merge_gate.sh` の 1 段目は「その head の CI が緑になるまで待つ」で、`collector-windows-runtime` は `crates/` の変更で必ず走る（ci.yml の changes `^(crates/|Cargo\.)` → windows=true）。この枝は crates/collector-windows を変えているので、9.2 の試験が緑でない限り merge されない。`[x]` は Task gate が grep を見て付けたもので、CI の担保は publish の後に gate が持つ（この席では push しない）

## security review（de2b737..02b7fd1）

見た経路:
- `GET /contract` → `contract_version::contract_get` → `authorize` → `Caller::ApiToken` 以外は 403、資格情報なしは 401。答えは定数の整数だけ（crates/server/src/contract_version.rs:56-66）（4）
- `POST /ingest` `/heartbeat` `/drops` `/media/missing` → router が `*_route` に差し替え（lib.rs:4163-4167）→ `authorize_versioned`（`authorize` → `check_header`）→ 本文の断り（`JsonRejection`）→ `ItemGate` → 既存の `*_one`。資格情報なしは本文・見出しに依らず 401 が先（4）
- `PUT` / `HEAD /media/originals/{sha}` → `authorize` → `check_header` → 既存の処理（media.rs:475-477, 797-799）。426 で原本を置かない（3）
- 見出し `Ashiato-Contract-Version` → `requested_version`（数字だけ・u32・1 以上。それ以外は 426）。値は SQL・パス・ログの文に入らない（ログは整数か -1）（3）
- 1 件の `contract_version` → `ItemGate`（i64 の範囲）→ `ingest_request` で `i32::try_from` → `.bind` で列へ。SQL の文字列に入らない（lib.rs:1422-1443, 1642）（3）
- 知らない欄の断り → 結果は種別と送った `id` だけ、ログは種別と版の定数だけ。欄の名前・値は応答にもログにも出ない（contract_version.rs:236-246）（3）
- 移行 202610300030 → 錠を外して `contract_version` だけ埋め戻し、同じまとまりで張り直す。消した記録の `raw` / `payload` に触れない（本文を戻す経路なし）（3）
- 収集アプリ（PC `sender.rs:382` / 携帯 `HttpTransport.get`）→ 既存の `base_url` と Bearer で `GET /contract`。新しい送信先・待ち受けなし。通知の文は固定の文と日数だけ（contract_notice.rs:63, ContractNotifier.kt:91-92）（3）
- 依存: Cargo.toml / Cargo.lock / gradle / package.json の差分なし（3）

安全の決め手: 差分が足した・差し替えた収集アプリの口は、どれも最初に `authorize` を通り、版の検査・本文の断り・1 件の検査はその後にしか走らない —— 確かさ 4。
`scripts/quiet-run sec -- cargo test -p ashiato-server --lib -- st50_contract_requires_token st50_contract_web_session_403 st50_header_426_before_body_rejection st50_header_web_session_exempt st50_unknown_field_web_session` → `5 passed; 0 failed`
（資格情報なし・違う合言葉の `/contract` は 401 で本文に `contract_version` なし、画面のログイン・衛星は 403、4 つの POST の口は見出しの版 3 と壊れた本文でも違う合言葉なら 401）。原本の PUT / HEAD の順は file:line で 3 まで。

指摘なし

## /code-review（de2b737..02b7fd1）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R21. run() leaves lastContract unchanged when it returns early (the answer is not Version, mayOpenFiles() is false, or another run is still going), so an old refused=true stays put, and LocationService line 988 calls contractGate.noteOriginalsRefused() again on every tick.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalUploader.kt:129
- 根拠: 1) An original gets a 426 from PUT, so lastContract.refused becomes true. 2) The user revokes location permission (MediaAccess.NO_LOCATION), or /contract keeps answering Transient. 3) The server is updated. From then on run() returns 0 without calling pass(), lastContract.refused stays true, and every afterRecords call sets the originalsRefused flag again. The next Version observe sets reasons.refused=true, so the gate keeps waiting and the 'サーバを更新してください' notification and on-screen line never go away, even though the server now accepts everything.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.1 （b65703b3。再現: run() は `limit == null` か `mayOpenFiles()` が偽のとき `lastContract` を書かずに返り、LocationService.kt:988 が前の回の refused で `noteOriginalsRefused()` を呼び続ける。開かずに終えた回は `ContractOutcome()` に戻す。St50SenderTest の 426 の試験に「位置の権限を外した回に前の 426 が残らない」を足した）

## R22. When GET /contract does not return a version (401/403/5xx/unreachable count as Transient), flush returns without sending and without logging, so the PC collector stops posting and writes nothing to telemetry.
- 成果物: crates/collector-windows/src/sender.rs:164
- 根拠: The API token is wrong or expired. GET /contract returns 401, read_contract_answer maps it to Transient, both flushes return Ok(0,0) with no log line, and ContractGate.observe(Transient) returns right away. Before this change the POST logged the 401 or send_failed. Now records pile up forever and the logs show no unauthorized or send_failed line. Android at least logs send_held.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 8.1 （b65703b3。再現: runtime.rs の send_at は Version 以外の答えで送り手を `None` で呼び、送り手は `Ok(0,0)` で返るのでログが 1 行も出ない。携帯の `send_held` と同じ語で `kind=send_held error=<noendpoint|unreadable|transient>` を契機ごとに 1 行残す）

## R23. nextBatch decides that a probe missing from the version-filtered head means the probe row is no longer in the outbox, but headStored also leaves out rows whose contractVersion is above upTo.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:228
- 根拠: A row with contract_version 2 is rejected with contract_unsupported and goes into versionRejected. Later /contract answers A=1. headStored(..., maxVersion=1) leaves the row out, fresh.size < MAX_BATCH, so the row is removed from versionRejected and skipped even though it is still pending. Reason (d) is cleared by mistake. It is only still covered when the ahead shortcut (limit < currentVersion()) happens to hold.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 再現の筋書きは「版 2 の行が contract_unsupported で断られた後に /contract が 1 を答える」で、版 2 の行を送るには前に A ≥ 2 を得ている必要がある —— A が下がる（S-01 を古い版へ戻した）ときだけ起きる。scoped re-review の Minor と同じもので、`.superpowers/sdd/tasks/progress.md` に Ruling つきで park 済み（D9 は受け付ける版が下がらない前提。代償は戻した間に待ちを 1 契機早く抜けるだけで、行は消えない）

## R24. Every send trigger now makes a blocking GET /contract on the collector's tick thread, even when both outboxes are empty. Before this change an empty outbox made no network call.
- 成果物: crates/collector-windows/src/runtime.rs:813
- 根拠: The server is slow or unreachable. Each send now blocks the foreground-window sampling loop for up to REQUEST_TIMEOUT_SEC before flushing, including idle ticks that have nothing to send. A cheaper version: skip the GET when events and beats are both empty, or ask only when a batch is about to be posted. Android's Drainer.tick also asks on every tick.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D18（仮）（design.md:181-184）が「5 分の送る契機の頭で 1 回 GET /contract」を選び、費用（5 分に 1 回の小さな GET）と反転条件（問い合わせの数が目に見える量になったら答えを 1 時間覚える）を書いて台帳に載せている。未送信が空になるのは記録が無い（人が PC を使っていない）間なので（記録は runtime.rs:793 の `engine.flush` で送る直前に積まれる）、見回りが止まって失うものも無い。届かない S-01 で時間切れまで待つのは、記録があるときの POST と同じ

## R25. On every send, `oldest` is computed by scanning the whole events outbox, with a hash lookup per row and an RFC3339 parse for rows missing from the enqueue ledger, even when the notice will do nothing.
- 成果物: crates/collector-windows/src/runtime.rs:850
- 根拠: After a long outage there are about 100k pending rows. Every send trigger (every few minutes) scans and parses them all, even when gate.waiting() is false or notified_at is less than 24h old, so the value is never used. Compute it lazily, only when decide() returns Show.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 （b65703b3。`after_send` の `oldest` を閉包で受け、通知を出す契機（`decide` が Show）でだけ未送信をなめる）

## R26. The backfill updates every c01-/c02- row in core.event and core.event_version in one statement, with triggers disabled through ALTER TABLE, which holds ACCESS EXCLUSIVE locks on both tables for the whole rewrite.
- 成果物: migrations/202610300030_contract_version.sql:82
- 根拠: On a production DB with millions of collected rows, the migration rewrites every matching row in both tables and creates that much dead-tuple bloat. Reads and ingest are blocked for the whole migration while the server starts. There is no batching and no way to use a default for old rows.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 移行はサーバの起動の中で当てない —— `ashiato-server migrate` で別に当て、`run()` は当たっていない版があれば起動を断る（lib.rs の `migration_ledger::startup_refusal`。design D5）。移行の間はサーバが待ち受けていないので、読み出しも取り込みも止める相手がいない。1 回きりの書き直しで、錠を外す窓を最小にするため 1 つのまとまりで当てる（D20）

## R27. The ItemRefusal→error-enum match and the X/X_route wrapper pair are copied with small changes into all 4 handlers (ingest, heartbeat, drops, media_missing).
- 成果物: crates/server/src/lib.rs:2057
- 根拠: Adding a new ItemRefusal variant means editing 4 hand-written matches. One `impl From<ItemRefusal>` per error enum, or a small generic helper that runs gate.check and maps the result, would remove about 60 duplicated lines and keep the 4 doors from drifting apart.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 4 つの match は網羅的なので、`ItemRefusal` に種類を足すと 4 か所とも型の検査で落ち、黙ってずれることは無い（`impl From` にしても 4 つの impl を書き直すのは同じ）。`X` と `X_route` の対は意図した形で、`X` は既存の試験が `Json` で呼ぶ口、`X_route` は本文の断りを資格情報と見出しの後へ回すための router の口（final review R8）

## R28. ingest_one (and the heartbeat handler at line 3200) still parse sent_id inline, duplicating the new contract_version::sent_id helper.
- 成果物: crates/server/src/lib.rs:1188
- 根拠: There are now two copies of the 'pick up the sent id even from a malformed item' logic. If one changes, for example to accept uppercase or braced UUIDs, the gate-rejected results and the malformed results report ids differently. Call contract_version::sent_id(item) instead.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.1 （b65703b3。ingest・heartbeat・drops・media_missing の 4 か所を `contract_version::sent_id` に寄せた）

## R29. Outbox::open runs rewrite() when broken lines exist but skips rewrite_enqueued(), so enqueue-ledger lines for sent or quarantined rows are not trimmed in that branch.
- 成果物: crates/collector-windows/src/outbox.rs:99
- 根拠: Opening an outbox that has one broken line takes the broken > 0 branch. The `else if enq_lines != len` cleanup is skipped, so the stale lines stay in outbox.enq.jsonl until a later open with no broken lines or a later remove() rewrites it. If broken lines keep showing up, the ledger keeps growing.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 8.1 （b65703b3。再現: `broken > 0` の分岐が `else if` の控えの書き直しを飛ばす。2 つを別の if にし、`st50_outbox_trims_enqueued_ledger_when_quarantining`）
