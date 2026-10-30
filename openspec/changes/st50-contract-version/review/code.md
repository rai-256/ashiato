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
