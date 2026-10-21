# final review — st13-scheduled-account-fetch

## final review（f655f3f..e15b9f3）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md` をそのまま）。review package
`.superpowers/sdd/tasks/review-f655f3f6..e15b9f34.diff`（37 commit）と、全 Task の `task-*-findings.md` の Minor と ⚠️ を見た。
判定: **With fixes**（Critical 0 / Important 2 / Minor 13）。

reviewer が閉じた ⚠️: Task 3（`check-scheduled-clock` に `Utc::now()` を入れると rc=1）・Task 10（`UnusedAppRestrictionsTest` tests=9 failures=0）・
Task 5（CI の runner は非 root なので chmod 000 の試験が効く）・Task 2（smoke は Task 7 の head で PASS）・Task 8（`testdb::source` は一意の接尾辞）。

## R1. `file_unreadable` になった書庫を中身の sha で「読んだもの」として覚えるので、画面の「置き直すか権限を確かめて」に従っても二度と読まれない
- 成果物: crates/server/src/archive/worker.rs, crates/server/src/archive/scan.rs
- 根拠: worker.rs:244-266 `record_unreadable` が outcome='unreadable' を残す / 専用のフォルダのファイルは取り込み済みへ移る / scan.rs:136-147 と worker.rs:2210-2213 の既読の EXISTS が `'unreadable'` を含む → 同じ中身を置き直すと AlreadyRead / web/src/messages.ts の文言「置き直すか、ファイルの権限を確かめてください」。Takeout は約 7 日で失効するので落とし直しても入らない
- kind: technical
- 処置: fixed D20 仮 — `eced51b4`。既読の判定（`REMEMBERED_OUTCOME`）から file_unreadable を外し、置き場に残す

## R2. 最初から読めない 1 ファイルで走査全体が落ち、両方の置き場が止まる（R34 の取り残し）
- 成果物: crates/server/src/archive/scan.rs
- 根拠: scan.rs:95-100 `hash(&file.path)?` が `std::fs::read` の失敗を走査全体の Err にする / scan.rs:182-192 の包括の `From` で列挙中の `entry?`・`metadata()?` も Other になり、もう片方の置き場も巻き込む / 生存信号は `inbox_scan_failed` だけで台帳にもファイル単位にも残らない。spec external-ingestion:304 の SHALL「開けない・読み出せない失敗を…台帳に残す」。Scenario（:386）は「見つけた後で」に絞られていて試験は緑
- kind: technical
- 処置: fixed D20 仮 — `eced51b4`。一覧・ハッシュの 1 件の失敗はそのファイルだけ台帳に残して飛ばし、置き場の一覧が取れないときだけ blockers

## R3. 見回りの行が無い・契機が interval でないと warn が 1 秒ごとに出続ける（Task 2 F3）
- 成果物: crates/server/src/archive/worker.rs:2062-2096
- 根拠: 1 日 86400 行
- kind: technical
- 処置: fixed 2.1 — `eced51b4`。間隔を読めない warn は状態が変わった 1 回だけ

## R4. 定期取得の枠: `join_next` の JoinError を捨てる（F1）・コメントが実際と違う（F5）・warn の `error = %error`（F2）・起動時に登録簿を読めないと再起動まで動かない（F3）
- 成果物: crates/server/src/scheduled.rs
- 根拠: Task 3 の task-3-findings.md F1/F2/F3/F5。F2 は Global Constraints「定期取得の枠のログは種別と名前だけ」から字面で外れる（中身は core.source の SQL エラーで私的データは出ない）
- kind: technical
- 処置: fixed 3.1 — `eced51b4`。JoinError を種別と名前で残す・登録簿を読めるまで読み直す・warn から本文を外す・注釈を直す

## R5. 試験の trigger と関数が冪等でない（Task 4 F1）
- 成果物: crates/server/src/testdb.rs:244-266
- 根拠: 試験が途中で落ちて残ると次の回は CREATE FUNCTION が常に落ちる
- kind: technical
- 処置: fixed 4.1 — `eced51b4`。testdb.rs の trigger と関数を冪等に（CREATE OR REPLACE / DROP … IF EXISTS）

## R6. 論理ソースごとの行の失敗も `kind=archive_ledger` になり、ログで原因を分けられない（Task 4 F3）
- 成果物: crates/server/src/archive/worker.rs:2576
- 根拠: task-4-findings.md F3
- kind: technical
- 処置: fixed 4.1 — `eced51b4`。論理ソースごとの行の失敗を kind=archive_ledger_source で出す

## R7. 注釈が「design D7 の 4 つ」のまま（Task 5 F5）
- 成果物: web/src/messages.ts:152
- 根拠: file_unreadable を足して 5 つ。D8 の併記が無い
- kind: technical
- 処置: fixed 5.2 — `eced51b4`。messages.ts の注釈を 5 つ（D7 / D8）に

## R8. 退役の試験が `last_archive_on` の null を assert していない（Task 8 F3）
- 成果物: crates/server/src/sources_tests.rs:189-199
- 根拠: task-8-findings.md F3
- kind: technical
- 処置: fixed 8.1 — `eced51b4`。sources_tests.rs の退役の試験に last_archive_on の null を assert

## R9. 鳴るちょうどの時刻に「あと 0 日」が出うる（Task 8 F5）
- 成果物: crates/server/src/sources.rs:46-60, web/src/sources.ts
- 根拠: task-8-findings.md F5
- kind: technical
- 処置: fixed D21 仮 — `eced51b4`。過ぎるまでは残り日数を 1 以上にし、画面も 1 未満を受けたら「過ぎた」側で出す

## R10. e2e の試験名が「通知まで 通知までの残り日数」と崩れている（Task 9 F8）
- 成果物: web/e2e/coverage-sources.spec.ts:155
- 根拠: 74 日 23 時間の試験は seed から 1 時間過ぎると「過ぎた」側しか見ない（CI は毎回 seed し直すので担保される）
- kind: technical
- 処置: fixed 9.2 — `eced51b4`。e2e の試験名を直した

## R11. クラスの doc が「画面は持たない」のまま（Task 10 F4）・`mapOf(...) + extraFields()` が同名の欄を上書きできる（F6）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:19, Heartbeat.kt
- 根拠: task-10-findings.md F4 / F6
- kind: technical
- 処置: fixed 10.1 — `eced51b4`。MainActivity の doc を直し、Heartbeat の足す欄が決まった欄を上書きしない（HeartbeatTest に 1 本）

## R12. 足した版の行にだけ説明のコメントが無い（Task 1 F5）
- 成果物: migrations/ORDER:58
- 根拠: task-1-findings.md F5
- kind: technical
- 処置: fixed 1.1 — `eced51b4`。migrations/ORDER の足した行に注釈

## R13. 縦串の台本: HANDS_SRV に trap が無い（F1）・`+100` の port が worktree st(n+50) と被る（F2）・visit しか見ない（F3）
- 成果物: tools/smoke.d/202610212100_st13-archive-hands-off.sh
- 根拠: task-7-findings.md F1/F2/F3
- kind: technical
- 処置: fixed 7.1 — `eced51b4`。F1: 2 つ目のサーバに trap / F2: port を worktree が持つ +1（tools/ports.sh）。F3（visit しか見ない）は残す —— 縦串の役は「手作業の範囲」の流れを本物の S-01 で通すことで、各ソースの中身は `archive_flow_tests.rs` の流れの試験が持つ

## R14. 試験が書庫の経路を通らない・この版だけを当て直す（Task 1 F1/F2）、THEN の生存信号を archive_scan_counter で代用（Task 5 F1/F4）
- 成果物: crates/server/src/source_acquisition_tests.rs ほか
- 根拠: reviewer は「許容の範囲」と判定
- kind: technical
- 処置: rejected: reviewer 自身が「許容の範囲」と判定。Task 1 F1 —— 試験が直接呼ぶ `ensure_myactivity_source` は書庫の読み手が呼ぶのと同じ関数（`crates/server/src/archive/worker.rs:1761` / `:2529`）。Task 5 F1/F4 —— 生存信号は `record_archive_heartbeat`（`worker.rs:1509`）が `core.archive_scan_counter` から組むので、その数を見ることは同じ源を見ている

## R15. 「定数をわざと変えた木で rc≠0」の結果を PR 本文に書く（Task 1 ⚠️）
- 成果物: tools/check-source-intervals.sh
- 根拠: task-1-findings.md の ⚠️。finish で PR 本文に載せる
- kind: technical
- 処置: fixed 1.2 — 一時コピーの木で `tools/check-source-intervals.sh` を走らせた: 変えない木 rc=0 / `FIX_INTERVAL_MS` を 120_000 にした木 rc=1（`NG c01-location: 登録簿 60 秒 != 収集側 120 秒`）/ さらに `HISTORY_INTERVAL` を 7 時間にした木 rc=1（c02-browser-history も NG）。PR 本文にも載せる

## R16. 退役と not_exempt の e2e を seed でなく応答の差し替えで作った（Task 9 F4。計画からの逸脱）
- 成果物: web/e2e/coverage-sources.spec.ts
- 根拠: 理由はコメントにあり妥当。tasks / PR 本文にも記録する
- kind: technical
- 処置: rejected: 直すものは無い —— 逸脱の理由は `web/e2e/coverage-sources.spec.ts:173` のコメント（退役の格子は畳まれて既存の書庫の格子の実寸の e2e を落とすので seed に置かない）と `:76`（device_notice だけ差し替え、行は本物）にあり、Scenario は本物の Chromium で担保される。計画からの逸脱としての記録は finish の PR 本文で行う

## scoped re-review（d4cb190..eced51b）

席: scoped re-review（SDD の `re-review-prompt.md`）。package `.superpowers/sdd/tasks/review-d4cb190c..eced51b4.diff`。
判定: **All findings addressed, no new Critical/Important breakage**。R1〜R12 は ADDRESSED、R13 は F1/F2 が ADDRESSED（F3 は処置どおり残す）、
R14〜R16 は fix の範囲外（処置どおり）。試験の抜け（壊れてはいない）: R3 の warn を 1 回にすること・R4 F3 の登録簿の読み直しの loop には試験が無い。
範囲外の観察: 書庫の台帳を残せないとき・`archive_scan_interval` の warn に `error = %error` が残る（sqlx のエラーで私的データは無い。定期取得の枠の外で fix 前からある）。

## R17. `file_unreadable-path:<パスの sha>` の鍵は中身を含まないので、同じ名前のファイルが 2 度目に最初から読めないと台帳にもログにも出ない
- 成果物: crates/server/src/archive/worker.rs:605-635
- 根拠: scoped re-review の New Breakage（Minor）。`record_unhashable` の鍵はパスだけで、一意索引 (user_id, sha256, parser_version, outcome) の `ON CONFLICT DO NOTHING` が 2 度目を落とし `rows_affected=0` でログも出ない。専用のフォルダの `Timeline.json` は名前が決まっているので起きうる。ダウンロードのフォルダは名前に時刻が入るので起きにくい。D20（仮）の「何度走査しても 1 行」はこの再発を考えていない
- kind: technical
