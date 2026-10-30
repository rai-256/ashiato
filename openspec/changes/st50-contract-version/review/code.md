# ST50 code review

## final review（87a0ab0..c4ffe79）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 1 / Important 4 / Minor 11。
reviewer が手元で走らせた結果: `cargo test -p ashiato-collector-windows --lib` 311 passed、`cargo test -p ashiato-server` 1587 passed / 1 failed
（`archive_scan_interval_change_takes_effect_without_restart`。単独で ok・ST50 は触っていない。時間に依る不安定な試験として R にしない）、
`check-openapi.sh` / `check-contract-version.sh` / `check-migrations.sh` OK。
Task ごとの ⚠️ のうち、Task 9（after_send / ContractToaster の配線）・Task 10（LocationService の 404・読めない答え）・Task 5（例の書き換え禁止は検査 4 が担う。D8）は解消済みと判定。

## R1. origin/main に入った ST49（`202610251000_record_component_version`）・ST23（`record_purge`）とぶつかる。rebase の後は移行の手順 3 が本物の DB で RAISE で落ち、ST49 の検査が黙って消える
- 成果物: migrations/202610300030_contract_version.sql
- 根拠: `git merge-tree` で 20 ファイルが衝突（`event_end_time.sql`・`ORDER`・`ingest.rs`・`lib.rs`・`drops.rs`・`media_missing.rs`・`export.rs`・`openapi.json`・`Outbox.kt`・`Sender.kt`・`IngestRequest.kt`・`contract.rs` ほか）/ 手順 3 の `copied_from` は ST41 の md5、門と前の版の錠の写しは ST41 の本体（ST49 の `collector_version` / `web_version` / `server_version` の検査を含まない）/ `event_version_live` に ST49 の列が無い / `.down.sql` は ST41 へ戻す / ST49 の守りは `copied_from = ARRAY['048663…']` だけを許し、ST49 の `CREATE OR REPLACE VIEW event_version_live` は `contract_version` を持たない / Task 4 の parked の Ruling が「ST49 の移行にも同じ形が要る」と既に書いている
- kind: technical

## R2. `unknown_field` / `contract_unsupported` で断られている間、新しい記録が受け付けられるたびに待ちを抜けて通知が消え、次の契機で出直す（PC・携帯）
- 成果物: crates/collector-windows/src/contract_gate.rs
- 根拠: `contract_gate.rs` の `observe` の Version の分岐 / `ContractGate.kt:observe`（`rejected` が偽になると飛ばす印と `notified_at` が捨てられる）/ deep.md Q5・Q6（止まった時に 1 回・1 日 1 回）/ D8（配った印が無い間は版を上げずに欄を足せる）/ Task 8 F3
- kind: technical

## R3. 携帯で再起動の直後に 24 時間未満の読めない 200 を得ると `known = true` になり、出ていた通知と画面の行が消える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: `ContractGate.kt` の Unreadable・NoEndpoint の分岐が `known = true` / `ContractNotifier.update` が通知と `store.notice`・`notifiedAtMs` を消す / Scenario「再起動しても 24 時間経つまで通知は出直さない」/ PC は `reasons_known` を Version の分岐でだけ立てる / 今の試験は「確かめる前」だけを見る
- kind: technical

## R4. `tools/check-openapi.sh` の `contract_version` の除外が残り、収集アプリの欄 ⊆ サーバの欄を機械で見る検査が無い
- 成果物: tools/check-openapi.sh
- 根拠: ingest・heartbeat・drops の 3 か所の除外（期限「携帯が送るようになるまで」は Task 10 で過ぎた）/ DropReport・Heartbeat の試験の `- contractVersionField` / `check-contract-version.sh` は「サーバの部分と openapi」「写しと印」しか比べない / Task 3 の ⚠️
- kind: technical

## R5. 最終の HEAD での全体の検証の証跡が無い
- 成果物: openspec/changes/st50-contract-version/evidence.jsonl
- 根拠: evidence.jsonl は絞り込んだ実行だけで head は 23 種（最終の c4ffe79c を含まない）/ Task 4 で「全体の 90 本が落ちる」とされた後のサーバ全体の記録が無い / 携帯の `testDebugUnitTest` 全体・detekt・`check-immutable.sh` は未実行 / R1 の rebase で全 SHA が変わる
- kind: technical

## R6. `ContractAnswer.contract_version` が openapi で `"minimum": 0`
- 成果物: crates/server/src/contract_version.rs
- 根拠: docs/openapi.json の `ContractAnswer` / 最初の印で口の形が凍結される（D8）/ Task 1 F1
- kind: technical

## R7. `ContractGate.kt` の `refused` が短絡評価で `originalsRefused.getAndSet(false)` を消費しない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: `(posted && …refused) || originalsRefused.getAndSet(false)` / Task 10 F4（`OriginalUploader.lastContract` が早く返った回に更新されない）
- kind: technical

## R8. 4 つの JSON の口は `Json` の抽出子が見出しの検査より先に本文を解釈する
- 成果物: crates/server/src/lib.rs
- 根拠: Task 2 の ⚠️（diff:469・504・535・764）/ 壊れた本文・大きすぎる本文・content-type 無しでは 426 でなく 400/413/415
- kind: technical

## R9. 携帯は待っている間、契機ごとに未送信を全部復号して走査する（最大 3 回）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt
- 根拠: `Sender.flush`・`SegmentStore.hasVersionAbove`（新しい版の行は末尾にあり毎回ほぼ最後まで読む）
- kind: technical

## R10. PC の通知の日数が積んだ時刻でなく `event_time` の最小から出る
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs` の `oldest`（ブラウザの履歴の古い訪問で日数が大きくなりうる）
- kind: technical

## R11. `ItemGate::new` が要求ごとに utoipa の schema を組み直し、`known_fields` の `unreachable!` が要求の経路にある
- 成果物: crates/server/src/contract_version.rs
- 根拠: Task 3 F2・F6
- kind: technical

## R12. `i32::try_from(v).ok().unwrap_or(1)` が i32 に収まらない版を黙って版 1 にする。移行の頭の注記が台帳の実装と合わない
- 成果物: crates/server/src/lib.rs
- 根拠: Task 4 F2・F3 / 移行の注記「`migrate()` は起動のたびに全版を当てる」（記録済みの版は飛ばす）
- kind: technical

## R13. `WaitStore::load` が壊れたファイルをログ無しで空として読む
- 成果物: crates/collector-windows/src/contract_gate.rs
- 根拠: Task 8 F6
- kind: technical

## R14. `contract_notice_skipped` が契機ごとにログに出続ける（PC・携帯）
- 成果物: crates/collector-windows/src/contract_notice.rs
- 根拠: Task 9 F5
- kind: technical

## R15. `docs/testing.md` の「16 本」の括弧の一覧に contract-version ×2 が無い
- 成果物: docs/testing.md
- 根拠: Task 7 F2
- kind: technical

## R16. smoke が `/tmp/st50-426.$$` を使い、curl が落ちると残る
- 成果物: tools/smoke.d/202610301200_st50-contract-version.sh
- 根拠: Task 12 F1 / HARNESS_TMP_LEAK
- kind: technical
