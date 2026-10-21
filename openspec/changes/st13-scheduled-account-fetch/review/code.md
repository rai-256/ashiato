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
- 処置:

## R2. 最初から読めない 1 ファイルで走査全体が落ち、両方の置き場が止まる（R34 の取り残し）
- 成果物: crates/server/src/archive/scan.rs
- 根拠: scan.rs:95-100 `hash(&file.path)?` が `std::fs::read` の失敗を走査全体の Err にする / scan.rs:182-192 の包括の `From` で列挙中の `entry?`・`metadata()?` も Other になり、もう片方の置き場も巻き込む / 生存信号は `inbox_scan_failed` だけで台帳にもファイル単位にも残らない。spec external-ingestion:304 の SHALL「開けない・読み出せない失敗を…台帳に残す」。Scenario（:386）は「見つけた後で」に絞られていて試験は緑
- kind: technical
- 処置:

## R3. 見回りの行が無い・契機が interval でないと warn が 1 秒ごとに出続ける（Task 2 F3）
- 成果物: crates/server/src/archive/worker.rs:2062-2096
- 根拠: 1 日 86400 行
- kind: technical
- 処置:

## R4. 定期取得の枠: `join_next` の JoinError を捨てる（F1）・コメントが実際と違う（F5）・warn の `error = %error`（F2）・起動時に登録簿を読めないと再起動まで動かない（F3）
- 成果物: crates/server/src/scheduled.rs
- 根拠: Task 3 の task-3-findings.md F1/F2/F3/F5。F2 は Global Constraints「定期取得の枠のログは種別と名前だけ」から字面で外れる（中身は core.source の SQL エラーで私的データは出ない）
- kind: technical
- 処置:

## R5. 試験の trigger と関数が冪等でない（Task 4 F1）
- 成果物: crates/server/src/testdb.rs:244-266
- 根拠: 試験が途中で落ちて残ると次の回は CREATE FUNCTION が常に落ちる
- kind: technical
- 処置:

## R6. 論理ソースごとの行の失敗も `kind=archive_ledger` になり、ログで原因を分けられない（Task 4 F3）
- 成果物: crates/server/src/archive/worker.rs:2576
- 根拠: task-4-findings.md F3
- kind: technical
- 処置:

## R7. 注釈が「design D7 の 4 つ」のまま（Task 5 F5）
- 成果物: web/src/messages.ts:152
- 根拠: file_unreadable を足して 5 つ。D8 の併記が無い
- kind: technical
- 処置:

## R8. 退役の試験が `last_archive_on` の null を assert していない（Task 8 F3）
- 成果物: crates/server/src/sources_tests.rs:189-199
- 根拠: task-8-findings.md F3
- kind: technical
- 処置:

## R9. 鳴るちょうどの時刻に「あと 0 日」が出うる（Task 8 F5）
- 成果物: crates/server/src/sources.rs:46-60, web/src/sources.ts
- 根拠: task-8-findings.md F5
- kind: technical
- 処置:

## R10. e2e の試験名が「通知まで 通知までの残り日数」と崩れている（Task 9 F8）
- 成果物: web/e2e/coverage-sources.spec.ts:155
- 根拠: 74 日 23 時間の試験は seed から 1 時間過ぎると「過ぎた」側しか見ない（CI は毎回 seed し直すので担保される）
- kind: technical
- 処置:

## R11. クラスの doc が「画面は持たない」のまま（Task 10 F4）・`mapOf(...) + extraFields()` が同名の欄を上書きできる（F6）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:19, Heartbeat.kt
- 根拠: task-10-findings.md F4 / F6
- kind: technical
- 処置:

## R12. 足した版の行にだけ説明のコメントが無い（Task 1 F5）
- 成果物: migrations/ORDER:58
- 根拠: task-1-findings.md F5
- kind: technical
- 処置:

## R13. 縦串の台本: HANDS_SRV に trap が無い（F1）・`+100` の port が worktree st(n+50) と被る（F2）・visit しか見ない（F3）
- 成果物: tools/smoke.d/202610212100_st13-archive-hands-off.sh
- 根拠: task-7-findings.md F1/F2/F3
- kind: technical
- 処置:

## R14. 試験が書庫の経路を通らない・この版だけを当て直す（Task 1 F1/F2）、THEN の生存信号を archive_scan_counter で代用（Task 5 F1/F4）
- 成果物: crates/server/src/source_acquisition_tests.rs ほか
- 根拠: reviewer は「許容の範囲」と判定
- kind: technical
- 処置:

## R15. 「定数をわざと変えた木で rc≠0」の結果を PR 本文に書く（Task 1 ⚠️）
- 成果物: tools/check-source-intervals.sh
- 根拠: task-1-findings.md の ⚠️。finish で PR 本文に載せる
- kind: technical
- 処置:

## R16. 退役と not_exempt の e2e を seed でなく応答の差し替えで作った（Task 9 F4。計画からの逸脱）
- 成果物: web/e2e/coverage-sources.spec.ts
- 根拠: 理由はコメントにあり妥当。tasks / PR 本文にも記録する
- kind: technical
- 処置:
