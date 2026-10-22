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
- 処置: fixed D20 仮 — `342895ba`。読み出せないファイルの鍵にパスと大きさと更新時刻を混ぜる（`io_failure_key`）。同じ名前で置き直した別のファイルは別の行（`archive_scan_a_new_unreadable_file_at_the_same_path_gets_its_own_row`）

## code_verify（ce3379e0）

席: code_verify（手 1〜7）。作業ツリーは clean（`git status --porcelain` が空）の HEAD `ce3379e0` で走らせた。壊したコードと DB の制約はすべて戻し、
戻した後の `git status --porcelain` / `git diff` は壊す前と同じ（違いはこのファイルだけ）。

### 申告と実測

申告: **13/13**（`- [x]` 13 件。ほかに人間の確認待ち 1 件「数か月操作しなくても権限が剥がれない」）。

| タスク | 検証コマンド（tasks.md） | 実測 | 判定 |
|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` | rc=0「マイグレーション OK」 | 一致 |
| 1.1 | `tools/check-immutable.sh` | rc=0（73 s） | 一致 |
| 1.1 | `CT source_acquisition (≥10)` | `--list` 16 本・16 ok | 一致 |
| 1.1 | `scripts/quiet-run unit -- cargo test -p ashiato-server` | 新しい DB で **rc=101**: 1258 passed / 1 failed（`export_file_tests::export_file_rows_heartbeat_has_no_attempt_counts_or_raw`、`ExportError { kind: Db, reason: "件数を数えられない: 40P01" }` = デッドロック）。単独で再実行 rc=0（1 passed）。DB のログに相手の文は無く、ST13 の変更に結び付く根拠は見つからなかった | **不一致 → R25** |
| 1.1 | `scripts/quiet-run smoke -- tools/smoke.sh` | rc=0（74 s） | 一致 |
| 1.2 | `bash tools/check-source-intervals.sh` | rc=0。一時コピーの木で `FIX_INTERVAL_MS` 30_000 → rc=1 / `HISTORY_INTERVAL` 12 時間 → rc=1 / `Duration::minutes(1440)` → rc=1（読めない）/ SQL の `c01-app-usage` 900 → rc=1 | 一致 |
| 2.1 | `CT archive_scan_interval (≥4)` | 4 本・4 ok | 一致 |
| 2.1 | `CT archive_ (≥1)` | 230 本・230 ok | 一致 |
| 2.1 | `grep -rn ASHIATO_ARCHIVE_SCAN_SEC … \| wc -l` が 0 | rc=0（根から。根以外で走らせても 0 件で rc=0 になる形なのは注 2） | 一致 |
| 2.1 / 7.1 / 9.2 | `bash tools/check-shared-appends.sh` | rc=0 | 一致 |
| 3.1 | `CT scheduled_ (≥9)` | 10 本・10 ok（台本の 2 本と合わせて Scenario 9 本） | 一致（ただし R19・R20） |
| 3.1 | `tools/check-scheduled-clock.sh`・`Utc::now()` を足した木で rc≠0 | rc=0 / `chrono::Utc::now()` を足した複製で rc=1 | 一致（ただし R18） |
| 4.1 | `CT archive_ledger_atomic (≥2)` | 3 本・3 ok | 一致 |
| 5.1 | `CT archive_inbox_independent (≥5)`・`CT archive_flow_names_only_the_unreadable_inbox (≥1)` | 7 本・7 ok / 1 本・1 ok | 一致 |
| 5.1 / 9.1 | `cd web && npx vitest run`・`npx tsc --noEmit` | 49 files / 479 passed rc=0・tsc rc=0 | 一致 |
| 5.2 / 9.2 | `cd web && npm run test:e2e -- coverage-sources`（17 本） | `STACK_RESET=1` で 17 passed rc=0 | 一致（ただし R22・R23） |
| 6.1 | `CT archive_failure_reset (≥2)` | 2 本・2 ok | 一致 |
| 7.1 | `CT archive_hands_off (≥3)`・smoke の `== Scenario:` 3 本 | 3 本・3 ok / 3 本の echo と「→ 印を置かずに 2 冊目が入った」「→ フォルダを作らずにタイムラインの訪問が 1 件入った」 | 一致 |
| 8.1 | `CT sources_ (≥9)`・`tools/check-openapi.sh` | 26 本・26 ok / rc=0「API の契約 OK」 | 一致 |
| 9.2 | `cd web && npm run test:e2e -- archive-layout coverage-` | `STACK_RESET=1` で 36 passed rc=0（`test.fail` の `archive-layout.spec.ts:156` は fail のまま） | 一致 |
| 10.1 | `GT '*UnusedAppRestrictions*' (≥7)`・`GT '*Heartbeat*' (≥1)` | `UnusedAppRestrictionsTest` tests=9 failures=0 / Heartbeat 系 5 クラス 29 本 failures=0（rc=0） | 一致 |
| 10.1 | `grep -c unused_app_restrictions docs/collector-contract.md` ≥ 1 | 4 | 一致 |

- 注 1: 1 回目の全体の走りは溜まった開発用 DB（`c01-location` の `collection_started_on` が別の試験の残した `2000-06-26`）で `day_tests::st25_reasons_before_collection_started` が落ち、
  エクスポートの照会が 1 本 2 分かかって 1 時間の上限で止められた。DB を作り直した 2 回目が上の行（3512 s）。実装側の前日のログ `20261021-225203-ct-all.log` も `FAILED. 754 passed; 505 failed`、
  `20261021-233726-ct-fresh.log` は完走の行が無い —— **1.1 の「全試験が通る」を rc=0 で示したログは手元に無い**。落ちた 1 本は ST13 の外（ST33 の書き出し）で、単独では通る
- 注 2: 2.1 の grep は、repo の根以外（`web/` など）で走らせると対象のディレクトリが無く 0 件になり rc=0 で緑になる（実測: `web/` から走らせて rc=0）。CI と tasks の書き方どおりに根で走らせれば問題ない

### 手 1: 固定値を独立に再計算する

一致（確かめた範囲: `sources_tests.rs` の `now() = 2026-10-21T03:00Z` からの 53 日 / 80 日 / 74 日 23 時間 / 75 日 1 時間 / 75 日ちょうどの
`last_archive_on`（Asia/Tokyo）・`alert_days_left`（切り上げ・最小 1）・`alert_passed` を python の `datetime` で計算 → `2026-08-29 / 22`・`2026-08-02 / 過ぎた`・`2026-08-07 / 1`・`過ぎた`・`1` で全部一致。
`sources.test.ts` の D11 の書き方（59 / 60 / 90 / 120 / 1800 / 3600 / 86400）を python で D11 の文の規則から作り直して一致。ハッシュを直書きした試験は ST13 の差分に無い）。

### 手 2: ガードをわざと壊す

- `check-source-intervals.sh` は壊した 4 通りすべてで rc=1（上の表）。移行の D3 の `UPDATE` から `AND s.acquisition IS NULL` を外した木で
  `changed_interval_survives_reapplying_the_migration` が落ちる（`FAILED. 15 passed; 1 failed`）—— 守りは効いている
- `check-scheduled-clock.sh` は名前の grep だけで、壁時計を読む別の書き方を通す → **R18**
- `check-immutable.sh` に ST13 の行は無い（`grep acquisition` 0 件）。登録簿の当て直しの守りは上の Rust の試験が持つ

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . st13-scheduled-account-fetch` → rc=0「scenarios: OK」（担保あり 1544 / 人間の確認待ち 1。ST13 の外の warn だけ）。
印の先を 1 件ずつ見て、主張の階層が違うもの: 定期取得の「S-01 を起動する」の 3 本（R19）、書庫の残り日数の画面の 2 本（R22・R23）。

### 手 4: 本人の決定が test で固定されているか

| 決定（deep.md / Q1 の既定） | 書き換えたときに落ちる試験 |
|---|---|
| Q3: 「手作業・2 か月ごと」・「通知まで あと N 日（最後の書庫 … から）」 | ある（`sources.test.ts` の `toBe(…)`、e2e の `toContain`） |
| Q3: S-7 設定に出さない・Q6: 変える操作を置かない | ある（e2e の S-7 の節の見出し・`h2` の中の `input/select/button/textarea` が 0） |
| Q4: 初回に 1 回だけ頼む・ボタン 24 dp | ある（`UnusedAppRestrictionsTest` の 2 回目の試験・`minDp >= 24f`） |
| Q1 の既定: 想定間隔より長い取得の間隔は断る | ある（21600 受け付け / 21601 断り。両側） |
| Q1 の既定: 見回りの初期値 120 秒 | ある（`assert_eq!(default, 120)`） |
| D3: 位置 60・アプリ利用 1800・時計 3600・Chrome 86400 | ある（`check-source-intervals.sh`） |
| D4: 見回りの上限 300 秒 | **無い → R21** |

### 手 5: tasks の `[x]` と実体

13 件とも検証コマンドが実在し、`CT` の絞り込みはどれも下限以上の本数がある（上の表）。`cargo test --test <名前>` の形は tasks に無い。
不一致は 1.1 の全体の rc（注 1・R25）と、9.2 の本文が書く seed とアサートが入っていないこと（R22）。

### 手 6: 隙間

記録を失う経路は、ST13 の差分の中では見つからなかった（確かめた範囲: 台帳の 1 トランザクション・読み出せない書庫を置き場に残す・中身の変わった書庫の回数の戻し・
専用のフォルダの作成・環境変数での起動の拒否は、試験と縦串がそれぞれ通る）。表示の誤りを 1 件（**R24**）、取り方だけ定期取得にした行が黙って飛ばされる経路を 1 件（**R26**）、実際に作って確かめた。

### 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st13-scheduled-account-fetch` が `[]`）。

## R18. 壁時計の 2 本の Scenario を守る `check-scheduled-clock.sh` は呼び出しの名前の grep で、`UNIX_EPOCH.elapsed()`・別名の `Utc`・`time` の crate・SQL の `now()` を通す
- 成果物: tools/check-scheduled-clock.sh / crates/server/src/scheduled.rs
- 根拠: `scheduled.rs` の複製の `run_source` の先頭に 1 行足し、`SCHEDULED_FILE=<複製> bash tools/check-scheduled-clock.sh` を走らせた: `let _w = chrono::Utc::now();` → rc=1（落ちる）/ `let _w = std::time::UNIX_EPOCH.elapsed();` → **rc=0** / `use chrono::Utc as Wall; let _w = Wall::now();` → **rc=0** / `let _w = time::OffsetDateTime::now_utc();` → **rc=0** / `sqlx::query_scalar("SELECT now()")` → **rc=0**。spec「壁時計が戻っても / 進んでも取りに行く回数は変わらない」の担保はこの台本の `echo` だけ（試験は `tokio::time` しか動かさない。design D5）。台本は落ちるときも先に 2 本の Scenario の行を「読む呼び出しが 0 件」と echo する
- kind: technical
- 提案: 呼び出しの名前でなく入口で止める（`scheduled.rs` の `use chrono` / `use time` / `std::time::{SystemTime, UNIX_EPOCH}` を禁じ、`now_utc` `UNIX_EPOCH` と SQL の `now()` / `current_timestamp` も数える）。Scenario の echo は判定の後に出す
- 処置: fixed 3.1 — `342895ba`。`check-scheduled-clock.sh` を入口で止める（chrono / time / SystemTime / UNIX_EPOCH / `Utc` の名前と SQL の now()・current_timestamp ほか）。指摘の 5 通りと `use std::time::SystemTime as W` の複製がすべて rc=1。Scenario の echo は判定の後

## R19. 定期取得の枠を S-01 の起動につなぐ 1 行を外しても、どの試験も落ちない（「S-01 を起動する」の 3 本の Scenario は `spawn_with` / `spawn` で止まっている）
- 成果物: crates/server/src/lib.rs:3580 / crates/server/src/scheduled_tests.rs
- 根拠: `lib.rs:3580` の `std::mem::forget(scheduled::spawn(pool.clone(), user_id));` をコメントにした木で `cargo test -p ashiato-server scheduled_` → rc=0、`10 passed; 0 failed`（`tests/server_startup.rs` の 7 本は定期取得を見ない）。縦串も定期取得の行を持たないので観測しない。spec の WHEN は「S-01 を起動する」（起動の直後に 1 回 / 取る手の無いソースは名前がログに出て S-01 は起動する / 0 本でも 1 分動き続ける）だが、試験は `scheduled::spawn_with` と `scheduled::spawn` を直接呼ぶ。いまは本番の取る手が 0 本なので、外れていても最初の取る手を足す Story まで誰も気づかない
- kind: technical
- 提案: 起動の配線を通る試験を 1 本足す（例: 起動の組み立てを関数に切り出し、`acquisition = 'scheduled'` の自分専用の行を置いて起動し、`scheduled_no_fetcher` の行に名前が出ることを見る）
- 処置: fixed 3.1 — `342895ba`。起動を `scheduled::start` に切り出し、実行ファイルを起動する `server_startup_starts_the_scheduled_fetch` が自分専用の行の名前を `scheduled_no_fetcher` で見る。`lib.rs` の呼び出しを外した木で FAIL

## R20. 起動時に登録簿を読めなければ読み直す（final review R4 F3 の直し・`REGISTRY_RETRY_SEC = 60`）を「1 回で諦める」に戻しても、`scheduled_` の 10 本は全部通る
- 成果物: crates/server/src/scheduled.rs:67-82 / crates/server/src/scheduled_tests.rs:314-337
- 根拠: `scheduled_rows` の失敗の枝の `tokio::time::sleep(Duration::from_secs(REGISTRY_RETRY_SEC)).await;` を `return;` に替えた木で `cargo test -p ashiato-server scheduled_` → rc=0、`10 passed; 0 failed`。`scheduled_registry` を名指す試験は `scheduled_with_no_scheduled_sources_runs_quietly` の「出ない」側だけ。scoped re-review も「R4 F3 の登録簿の読み直しの loop には試験が無い」と書いたが R になっていない。DB が S-01 より遅れて立ち上がる日に、再起動まで定期取得が止まる退行を落とせない
- kind: technical
- 提案: 登録簿を読めない状態（例: 権限を外した接続か、読めない pool）で `spawn_with` を起こし、読めるようにしてから `tokio::time::advance(60 s)` で取る手が 1 回呼ばれることを見る試験を足す。R19 の起動の配線を通す試験を、読めない pool → 読める、の順で組めば（`scheduled_registry` の info「読めた」が出る）1 本で両方を拾える
- 処置: fixed 3.1 — `342895ba`。`spawn_failing_registry`（試験の口）で読みを 2 回失敗させ、60 秒の読み直しの後に 1 回取りに行くことを見る（`scheduled_rereads_the_registry_until_it_can`）。sleep を return にした木で FAIL

## R21. 見回りの間隔の上限 300 秒（design D4）は 301 が断られることしか見ておらず、上限を 150 秒に狭めても全部通る
- 成果物: migrations/202610211000_source_acquisition.sql（`source_archive_inbox_interval_check`）/ crates/server/src/archive_scan_interval_tests.rs:268-291 / crates/server/src/source_acquisition_tests.rs:182-189
- 根拠: テスト用 DB の制約を `fetch_interval_sec <= 150` に作り直して `cargo test -p ashiato-server -- source_acquisition archive_scan_interval` → rc=0、`20 passed; 0 failed`（制約は `<= 300` に戻した。`pg_get_constraintdef` で確認）。300 が受け付けられる試験が無い（`grep 'fetch_interval_sec = 300'` 0 件）。300 は「置かれてから 10 分以内に読み始める」（間隔の 2 倍 ≤ 10 分）から決めた値で、本人が見回りを遅くしたいときに効く上限。docs/testing.md §3「その値を書き換えたときに落ちるテスト」
- kind: technical
- 提案: `s01-archive-inbox` を 300 にできて元に戻る試験を 1 本足す（`assert_refused` の対。transaction の中で行い共有の行は戻す）
- 処置: fixed 2.1 — `342895ba`。`archive_scan_interval_of_300_is_accepted`（transaction の中で 300 にして戻す）

## R22. 「最後の書庫が 53 日前 → 通知まで あと 22 日」の画面の Scenario は、seed に 53 日前の書庫が無く、e2e は応答の値をそのまま期待にしている（tasks 9.2 の seed とアサートが入っていない）
- 成果物: web/e2e/coverage-sources.spec.ts:115-128 / tools/seed.d/202610212200_st13-sources-grid.sh:5 / openspec/changes/st13-scheduled-account-fetch/tasks.md（9.2）
- 根拠: `STACK_RESET=1` の stack で `GET /sources` → `c03-youtube-watch` は `last_archive_on=2026-09-19`・`alert_days_left=42`（ST12 の seed の書庫のまま。seed の注釈「視聴履歴 seed.sh（ST12）の書庫の行（2026-09-19）がそのまま持つ。e2e は応答の値から期待を作る」）。その stack で e2e `視聴履歴は手作業・2 か月ごとと…` は passed。期待は `row.alert_days_left` と `row.last_archive_on`（応答）から作り、`alert_passed` なら「過ぎた」の枝に入るので、2026-09-19 + 75 日 = 2026-12-03 を過ぎると同じ e2e は「通知の日を過ぎた」を期待する枝に入る（同じ形の分岐が、Scenario の主張と逆の画面で緑になることは R23 で実測した）。tasks 9.2 は「視聴履歴を運んだ最後の書庫が 53 日前」の seed と「通知まで あと 22 日」のアサートを書いて `[x]`（`6a6a19c3`「e2e の書庫の期待を応答の値から作り、共有 seed.sh の書き換えを戻す」で外れた。理由は「seed した時点の now() と走らせた時点の差で偽の赤になる」）。数値は API の試験（`sources_archive_53_days_ago_shows_manual_and_days_left`）が固定しているが、画面の側で「22 日」と起点の日が並ぶことはどの試験も見ていない。R16（rejected）は退役と not_exempt を応答の差し替えで**置く**ことを認めたもので、ここは Scenario の WHEN（53 日前）が作られず THEN（あと 22 日）が観測されていない話
- kind: technical
- 提案: ST13 の seed（`tools/seed.d/…st13-sources-grid.sh`）に、視聴履歴と別の書庫のソース（または `page.route` で `/api/sources` の 1 行）を 53 日前で置き、「通知まで あと 22 日（最後の書庫 <今日−53 日> から）」を固定の文字で見る。tasks 9.2 の本文を実際の形に合わせる
- 処置: fixed 9.2 — `342895ba`。`/api/sources` の視聴履歴の行を 53 日前の値（2026-08-29・あと 22 日。API の試験 `sources_archive_53_days_ago_shows_manual_and_days_left` が固定）に差し替え、「通知まで あと 22 日（最後の書庫 2026-08-29 から）」を固定の文字で見る。tasks 9.2 の本文（seed に置く）とは形が違う —— 凍結の tasks は書き換えず PR 本文に書く

## R23. 「通知が鳴る 1 時間前 → 残り 1 日」の e2e は、seed から 1 時間を過ぎると「通知の日を過ぎた」が出ている画面で緑になる
- 成果物: web/e2e/coverage-sources.spec.ts:141-153
- 根拠: 立てた stack で台帳の `c03-timeline-visit` の書庫（`sha256 = repeat('4',64)`）の `created_at` を 2 時間古くし（seed から 2 時間経った状態。`session_replication_role = replica` で追記のみの trigger を外して 1 行だけ。stack の DB は `STACK_RESET=1` で作り直す捨ての DB）、`GET /sources` → `alert_days_left=null`・`alert_passed=true`。その状態で `npx playwright test coverage-sources -g '74 日 23 時間前'` → **1 passed**。試験は `if (row.alert_passed) expect(note).toContain("通知の日を過ぎた")` の枝に入り、Scenario の THEN（「あと 1 日」「過ぎたことは出ない」）を観測しない。手元で stack を立てたまま 1 時間後に e2e を回す（確認バッチの `run.sh` と同じ使い方）と常にこの枝になる
- kind: technical
- 提案: 境目の 2 本（1 時間前 / 1 時間後）は seed の時刻に依らせず、`page.route` で `/api/sources` の行を `alert_days_left: 1, alert_passed: false` / `alert_passed: true` に差し替えて画面の文字を見る（数値は CT が固定している）。分岐を残すなら、seed からの経過が 1 時間を超えたら skip でなく fail にする
- 処置: fixed 9.2 — `342895ba`。1 時間前の行を `alert_days_left: 1, alert_passed: false` に差し替え、「通知まで あと 1 日（最後の書庫 2026-08-07 から）」と「過ぎた」が無いことを見る。seed からの経過で枝が変わる形をやめた

## R24. 稼働状況の休止の知らせは、端末の時計（`emitted_at`）が進んでいた古い生存信号に負ける
- 成果物: crates/server/src/sources.rs:124-134
- 根拠: 立てた stack で利用者 `…00c7` に `c01-location` の生存信号を 2 件入れた: (a) `emitted_at = now()+1 日`・`received_at = now()−1 時間`・原文 `not_exempt`（時計が 1 日進んでいた端末）/ (b) `emitted_at = now()`・`received_at = now()`・原文 `exempt`（時計を直し、除外も済ませた後の信号）。`GET /sources?user_id=…00c7` → `device_notice = {'unused_app_restrictions': 'not_exempt'}`。`ORDER BY emitted_at DESC LIMIT 1` が端末の時計で選ぶので、本人が除外を済ませても、進んでいた時刻に実時間が追いつくまで位置の見出しは「休止の対象（アプリで外す）」のまま。この Story は時計のずれを `c01-clock` で別に扱っている（端末の時計は信用しない前提）。記録は失われず、追いつけば直る
- kind: daily
- 提案: 最新の信号を `received_at DESC`（同時刻なら `emitted_at`）で選ぶ。Outbox の送り直しで古い信号が後から届く場合を考えるなら、`received_at` と `emitted_at` のどちらで選ぶかを design D10 に 1 行書く
- 処置: fixed D23 仮 — `342895ba`。最新は `received_at DESC, emitted_at DESC` で選ぶ（`sources_device_notice_follows_the_latest_received_heartbeat`。`emitted_at` に戻した木で FAIL）

## R25. 1.1 の「`cargo test -p ashiato-server` rc=0」を示すログが無く、新しい DB での全体の走りは rc=101（書き出しの照会がデッドロック 40P01）
- 成果物: crates/server/src/testdb.rs（`fail_ledger_source_insert` / `allow_ledger_source_insert`）/ crates/server/src/export_file_tests.rs:53 / tasks.md 1.1
- 根拠: `docker compose down -v` → `up` → `tools/db-roles.sh` の新しい DB で `scripts/quiet-run ctfresh -- cargo test -p ashiato-server` → rc=101、`FAILED. 1258 passed; 1 failed; … finished in 3511.99s`。落ちたのは `export_file_tests::export_file_rows_heartbeat_has_no_attempt_counts_or_raw`（`ExportError { kind: Db, reason: "件数を数えられない: 40P01" }`）。DB のログは `deadlock detected at character 754`（書き出しの照会の `core.event_live` のあたり）で、相手の文は残っていない（DETAIL は client にだけ返る）。同じ 1 本を単独で `cargo test -p ashiato-server export_file_rows_heartbeat_has_no_attempt_counts_or_raw` → rc=0（1 passed、212 s）。実装側のログも `20261021-225203-ct-all.log` が `FAILED. 754 passed; 505 failed`、`20261021-233726-ct-fresh.log` は完走の行が無い。**ST13 の変更が原因とは確かめていない。** 確かめていない入口: この change が共有の DB に試験中の DDL を足したのは `testdb::fail_ledger_source_insert` / `allow_ledger_source_insert` だけで、`core.archive_ledger_source` に `CREATE TRIGGER`（ShareRowExclusive）/ `DROP TRIGGER`（AccessExclusive）を 1258 本と並走して取る。`LAST_ARCHIVE_AT_SQL` は `archive_ledger_source` を join する
- kind: technical
- 提案: `log_lock_waits = on`（か `deadlock_timeout` を伸ばす）の DB で全体を走らせ直し、デッドロックの相手を DB のログに残して原因を決める。ST13 の DDL が相手なら、trigger の差し込みを `stay_tests::fresh_db` 側へ移すか `archive_ledger_atomic` を直列にする。1.1 の `[x]` は rc=0 の走りを取り直してから
- 処置: rejected: 前提の「rc=0 のログが無い」は `evidence.jsonl` の 1.1（2026-10-21T09:11、head `b90f6220`、3434 s、`.superpowers/sdd/logs/20261021-181155-evidence-1.1-2-98691d.log` に `1219 passed; 0 failed`）で反証。finish の fix の後（`342895ba`）に `docker compose down -v` → `up` → `tools/db-roles.sh` の新しい DB を `log_lock_waits = on`・`log_error_verbosity = verbose` にして `scripts/quiet-run ctfresh -- cargo test -p ashiato-server` を走らせ直し rc=0（lib 1268 passed / server_startup 9 passed、3807 s。`.superpowers/sdd/logs/20261022-104410-ctfresh.log`）。DB のログにデッドロックも 1 秒を超えるロック待ちも 0 件で、ST13 の試験中の DDL（`fail_ledger_source_insert`）が相手という入口は再現しなかった。再発したら同じ設定の DB で相手の文を取ってから直す（設定は走りの後に戻した）

## R26. 取り方だけ「定期取得」にして契機を省いた行は、取りにも行かれず、`scheduled_no_fetcher` のログにも出ない
- 成果物: crates/server/src/scheduled.rs:130-136 / migrations/202610211000_source_acquisition.sql（`core.source_fetch_trigger_default`）
- 根拠: テスト用 DB で `INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind, acquisition) VALUES ('test-sched-silent','x',3600,'none','scheduled') RETURNING …` → `scheduled|derived|`（D2 の trigger が契機を `derived`・間隔なしで入れる。制約にも当たらない）。枠が起動時に読む条件 `acquisition = 'scheduled' AND fetch_trigger = 'interval'` でその行を数えると **0 件**（行は DELETE で戻した）。枠の取る手の照合とログ（`scheduled_no_fetcher`）はこの照会の行にだけ回るので、この行は名前もログに出ない。spec「登録簿に取り方が『定期取得』の論理ソースがあり、S-01 にその論理ソースを取る手が無い THEN … 論理ソースの名前をログに出す」。後の Story が登録簿に 1 行足すとき、契機を書き忘れると黙って動かない
- kind: daily
- 提案: `acquisition = 'scheduled'` なら `fetch_trigger = 'interval'` を求める CHECK を足す（定期取得は間隔ごとだけ）か、枠の照会を `acquisition = 'scheduled'` だけにして、契機が `interval` でない行を `scheduled_no_fetcher` と別の種別でログに出す
- 処置: fixed D22 仮 — `342895ba`。枠は `acquisition = 'scheduled'` の行を全部読み、契機が interval でない行は `scheduled_no_interval` で名前を出して飛ばす（`scheduled_source_without_interval_trigger_is_named_in_the_log`）

## security review（f655f3f..ce3379e）

見た経路:
- `GET /sources` → `router` の `route_layer(access_log::middleware)`（lib.rs:3496。`/sources` は `is_ingest` に当たらないので読み出しの記録を書く）→ `sources_get` の `authorize`（lib.rs:3134）→ `sources::read`（SQL は定数と bind だけ。外の値は `user_id` の UUID だけ）→ JSON（4）
- `GET /sources` の `device_notice` ← `core.heartbeat.raw`（端末が送る）→ serde_json で `unused_app_restrictions` の文字列だけを取り出す → 画面は `sources.ts` で `=== "not_exempt"` の比較にだけ使い、React の文字で出す（`dangerouslySetInnerHTML` なし）（3）
- 書庫の置き場の走査 `scan.rs` `list_dir` → 読めない 1 件は `record_unhashable`（台帳の鍵はパスの sha256。パスそのものは持たず `file_name` だけ）。ダウンロードのフォルダは `takeout-*.zip` だけが候補（scan.rs:275）なので、本人の他のファイル名は台帳に入らない（2）
- 置き場が読めないときのログ `error = %error` は `std::fs::read_dir` の `io::Error`（パスを含まない）。以前の `with_context(…dir.display())` のパス入りの文は消えた（2）
- 台帳の `read` 行と論理ソース別の行を 1 トランザクションにした結果、失敗した書庫は置き場に残り次の走査で読み直される → 記録の格納は内容の鍵で `DuplicateOfDeleted` になり、削除した記録は戻らない（4）
- `file_unreadable` の書庫を「覚えない」・動かさない → 読めるようになった次の走査で読まれる。中身はまだ一度も入っていないので戻る削除は無い（3）
- 定期取得の枠 `scheduled.rs`：本番の取る手は空（`fetchers()` が `Vec::new()`）。外への新しい送信先・listen は無い。ログは種別と論理ソースの名前だけ（2）
- 縦串の 2 つ目のサーバ（smoke.d/202610212100…）は `127.0.0.1` に bind（3）
- Android：`MainActivity` は設定の画面を開く `Intent`（自分の package 名）を足しただけ。manifest・exported の変更なし、依存の追加なし。生存信号の原文に `unused_app_restrictions`（4 値の列挙）を足す — 記録の中身ではない（2）
- 依存：`crates/server` の `[dev-dependencies]` に `tokio`（workspace と同じ版）の `test-util` を足しただけ。本番のビルドには入らない（2）

安全の決め手: 新しい入口 `/sources` は router の `route_layer` とハンドラの `authorize` を必ず通る —— 確かさ 4。`scripts/quiet-run unit -- cargo test -p ashiato-server --lib -- sources_without_credentials_is_401 archive_flow_a_deleted_record_never_returns archive_ledger_atomic_next_scan_rereads archive_ledger_atomic_failed_source_row` → 4 passed（`sources_without_credentials_is_401_without_list`：資格情報なしで 401、本文に `logical_source` が出ない。`archive_flow_a_deleted_record_never_returns_from_another_archive`：同じ中身を読み直しても削除した記録が戻らない）

指摘なし

範囲外: `GET /sources` の `user_id` は呼び手と結び付けずに受け取る（`outages_get` などほかの読み出しと同じ既存の形。利用者が 1 人の前提）

## /code-review（f655f3f..ce3379e）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R27. record_unhashable keys the ledger row only by path (`file_unreadable-path:<sha(path)>`), so the unique index allows one row per path for ever; a later, different archive at the same path that also can't be read gets no new row, and nothing tells the user.
- 成果物: crates/server/src/archive/worker.rs:394
- 根拠: takeout.zip in the inbox fails to hash once (row inserted), is later fixed and read. Weeks later the user drops a new takeout.zip with the same name and it is unreadable again (permissions). The INSERT hits ON CONFLICT DO NOTHING, no warn is logged, and /archives/status keeps showing the earlier successful read. The new archive sits there unread with no visible failure, and Takeout links expire after about 7 days.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D20 仮 — `342895ba`。R17 と同じ直し（鍵に大きさと更新時刻）

## R28. note_unhashable runs on the first sighting, before the two-scan stability check, so a file that is still being copied or is briefly locked gets a permanent `file_unreadable` ledger row.
- 成果物: crates/server/src/archive/scan.rs:162
- 根拠: The user copies a large zip into the inbox on Windows. The first scan finds it while the copier still holds it open, so hash_file fails with a sharing violation and a `file_unreadable` row is recorded. The screen briefly says 'ファイルを読み出せませんでした…権限を確かめてください' for a healthy file. Because that row is keyed by path, it also suppresses any real failure later at the same path (see the previous finding).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D20 仮 — `342895ba`。ハッシュの失敗は大きさと更新時刻が前の走査と同じときだけ台帳に残し、初めて見た・変わったときは sighting に大きさと更新時刻だけを書いて待つ（`archive_scan_a_briefly_locked_file_leaves_no_unreadable_row`。`if stable` を外した木で FAIL）

## R29. scheduled_rows and read_interval do not filter `retired_on IS NULL`, so a retired scheduled source is still fetched at every interval.
- 成果物: crates/server/src/scheduled.rs:1174
- 根拠: A later Story adds a scheduled source and then retires it (retired_on set, row kept). After the next restart, spawn_with still starts a task for it and it keeps calling the external fetcher, writing records for a source that the screen and sources.rs treat as retired.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 — `342895ba`。起動時と読み直しの照会に `retired_on IS NULL`（`scheduled_retired_source_is_not_fetched`）

## R30. The scheduled-fetch framework only starts inside the `if let Some(user_id) = archive_config.user_id` branch, which ties every scheduled source to the archive importer's configuration.
- 成果物: crates/server/src/lib.rs:3566
- 根拠: A deployment without ASHIATO_ARCHIVE_USER_ID (the archive importer is off) never starts scheduled::spawn. Once a later Story registers a fetcher, that source silently never fetches. The only log line is `archive_disabled`, which says nothing about scheduled fetching.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 — `342895ba`。design D5 のとおり利用者は取り込み器と同じ `ASHIATO_ARCHIVE_USER_ID` のまま、未設定なら `scheduled_disabled` をログに出す（`server_startup_without_archive_user_says_scheduled_fetch_is_off`）。利用者の設定を分けるのは最初の取る手を足す Story

## R31. Once started, a run_source task never stops; if its row is changed away from `scheduled`/`interval`, read_interval fails each cycle, the old interval keeps being used, and an unthrottled warn is logged every cycle.
- 成果物: crates/server/src/scheduled.rs:1208
- 根拠: A source's registry row is changed from fetch_trigger='interval' to 'manual' while the server is running. The task keeps fetching at the old interval and logs 'scheduled_interval 取得の間隔を読めない' every cycle until restart. That is the same log flooding R3 fixed for the archive scanner.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 — `342895ba`。読み直しで行が定期取得・間隔ごと・退役していない、でなくなったら `scheduled_stopped` を 1 回出して task を終える。DB の失敗の warn は読めなくなった 1 回だけ（`scheduled_stops_when_the_row_is_retired`）

## R32. When either inbox folder can't be read, sighting cleanup is skipped for both, so rows for files removed from the readable folder are kept.
- 成果物: crates/server/src/archive/scan.rs:177
- 根拠: Downloads is unreadable for a day while the user deletes and re-adds files in the dedicated inbox. The stale sighting rows (size, mtime, hash) stay. A file that reappears at the same path with the same size and mtime counts as 'stable' on its first sighting and skips the two-scan wait.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — `342895ba`。片方の置き場が読めないときも、読めた置き場（親が一致するパス）から消えた sighting は消す（`archive_scan_forgets_files_gone_from_the_readable_inbox`）

## R33. askToExemptOnce sets the 'asked' flag before showing the dialog, and start() opens the usage-access settings just before it, so the one automatic prompt can be hidden behind that screen or lost when the activity is recreated.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:187
- 根拠: On first launch, sendToUsageAccessOnce opens the settings screen and the exemption dialog is built behind it. If the activity is recreated while the user is in settings (rotation or process death), the dialog is gone but the flag is already set. The automatic explanation never appears, and the only remaining hint is the one-line text.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.1 — `342895ba`。印は本人が答えたとき（ダイアログが閉じたとき）に立て、答える前に画面が作り直されたら立てない。出している間は重ねて出さない（UnusedAppRestrictionsTest に 2 本。design D12（仮）を書き換えた）

## R34. wait_for_next_scan queries the registry once a second for the whole wait, about 86,400 extra DB queries a day per server only to notice a changed interval.
- 成果物: crates/server/src/archive/worker.rs:587
- 根拠: With the default 120 s interval, every scan cycle runs about 120 SELECTs on core.source. That is constant DB load and wakeups on a home PC. Reading the interval once per scan, or sleeping min(remaining, N) seconds, would be just as responsive for the smoke test's 1 s setting.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D4 の決定（spec「変わってから 1 秒以内」。寝る直前に 1 回読むだけだと縮めても最大 120 秒効かない —— spec-review R10）の帰結。読むのは loopback の DB の主キー 1 行で、`min(残り, N)` で眠ると縮めた間隔が最大 N 秒効かない。負荷が問題になったら D4 ごと人間に問う

## R35. LAST_ARCHIVE_AT_SQL is pasted into the SELECT twice, so its correlated subqueries (max over archive_ledger, and max(ingest_time) over core.event) run twice per source on every page load.
- 成果物: crates/server/src/sources.rs:1335
- 根拠: On each load of the coverage page, GET /sources scans core.event for max(ingest_time) twice for every archive source. With many events this doubles the cost of the slowest part. Computing last_archive_at once (subquery or LATERAL) and deriving the date from it, or converting with DAY_TZ in Rust, removes the duplicate.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 8.1 — `342895ba`。最後の書庫の時刻を副問い合わせで 1 回だけ導き、日付はその列から作る（sources_ の試験 26 本が通る）

## R36. read_scan_interval duplicates scheduled::read_interval (same query shape and error text) and has a different floor rule; record_already_read also writes out part of REMEMBERED_OUTCOME by hand.
- 成果物: crates/server/src/archive/worker.rs:567
- 根拠: There are two copies of 'read the interval from the registry': the archive one has no MIN_INTERVAL_SEC floor, the scheduled one has it. The file_unreadable exclusion is also written in two places. A future change to the outcome rules, or a lower interval constraint, has to update both, and missing one brings back the R1 bug where an unreadable file is remembered as read.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 — `342895ba`。既読の判定の `file_unreadable` を外す条件を `read_outcome!` の 1 か所にし、`REMEMBERED_OUTCOME` と `record_already_read` がそこから組む。間隔の読みの重複は rejected: 書庫の行は取り方が `server` で、定期取得の条件（`acquisition = 'scheduled'`・退役・止まる）と違う。下限の違いは DB の制約 `fetch_interval_sec BETWEEN 1 AND expected_gap_sec` で 1 未満が来ないので実害が無い

## scoped re-review（ce3379e..342895b）

席: scoped re-review（SDD の `re-review-prompt.md`）。package `.superpowers/sdd/tasks/review-ce3379e0..342895ba.diff`。
判定: **All findings addressed, no new Critical/Important breakage**。R17〜R24・R26〜R33・R35 は ADDRESSED、R34 の rejected と R36 の部分の rejected は ruling sound、R25 は全体の走りで判定。
New Breakage は Minor 3 件（直さない）: `spawn_reading` の試験用の `failing_reads` の分岐は本番では常に 0 / `server_startup_starts_the_scheduled_fetch` が途中で落ちると自分専用の `test-sched-` 行が残る（`scheduled_with_no_scheduled_sources_runs_quietly` は `test-sched-` を数えない）/ 読めない置き場がある走査で sighting を全件読む（件数は小さい）。

