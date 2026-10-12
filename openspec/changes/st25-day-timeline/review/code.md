# ST25 下流のコードレビュー

## final review（11b7d1f..b7f9fa1）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-11b7d1f4..b7f9fa11.diff`（ブランチ全体。ST06 / ST22 / ST28 の確認バッチの取り込みを含む）と、
ST25 自身の commit が触ったファイルに絞った `.superpowers/sdd/tasks/review-st25-scoped-11b7d1f4..b7f9fa11.diff`。
全 Task の ledger（Ruling は Task 10 の 1 件）と、全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **With fixes**（Critical 0 / Important 3 / Minor 11）。

申し送り（`docs/handoff/ST25.md`）は開始時から増えていない（最後の変更は e63ba335）。ST21・ST22 はまだ archive されていないので、
S-2 の見出しの `fix/` は PR 本文の「後続」に挙げる条件をまだ満たさない。

## R1. main と統合できない —— 画面の 4 ファイルが衝突し、main の文言の lint（`messages.ts` の外の直書きの禁止）で ST25 の画面の文言が落ちる
- 成果物: web/src/timeline.ts・web/src/DayView.tsx・web/src/Root.tsx・web/src/App.tsx・web/src/MasterView.tsx・docs/handoff/ST14.md
- 根拠: `git merge-tree --write-tree --name-only HEAD origin/main`（90952ed3）→ CONFLICT 5 件（docs/handoff/ST14.md・web/src/App.tsx・DayView.tsx・MasterView.tsx・Root.tsx）。
  main の 56d45374 / e2f459ed / fecc1327（`web/eslint.config.js`）が ASCII の外の文字の直書きを禁じ、merge した木で `npx eslint src/timeline.ts` が 36 errors（final reviewer の実測）。
  main の c20ef7ee が `scheme.ts` / `surface.ts` / `tokens.ts` を作り直し、ST25 の `useScheme` の置き場と重なる
- kind: technical
- 処置: fixed 11.2 —— 9cce10be でorigin/main（90952ed3）を merge し衝突 5 件を解いた。明暗は main の `scheme.ts` に一本化し、ST25 の文言は `messages.ts`（`day` / `timeline`）へ。`npm run lint` rc=0・vitest 235 passed・e2e 49 passed

## R2. 日付をまたぐ行で、要約（行の実際の範囲）と開いた中身（その日の端で切る）が食い違う
- 成果物: crates/server/src/day_records.rs:600-602・web/src/DayView.tsx:272
- 根拠: `records()` が明示の `from` / `to` も `day_bounds(day)` に収める。design D7「行の範囲は**行の実際の始まりから終わりまで**」・spec「行を開くと、その行の時間に入る…記録を出す」（spec.md:284）と矛盾。
  例: 09-07 23:00〜09-08 02:00 の滞在は、09-08 の要約に 23:30 の Code が入るが、開くと 00:00 以後しか出ない。開いた中身を固定する試験が無い
- kind: technical
- 処置: fixed D7 —— 明示の `from` / `to` は日の端で切らない（28c714c0。`st25_api_records_explicit_range_is_not_clamped_to_the_date` と st25-open の jsdom）

## R3. Q7 で名前を変えた Scenario の jsdom 側の印が無く、古い振る舞い（末尾に「ほかの記録」の行）を固定した試験が残っている
- 成果物: web/src/__tests__/st25-summary.test.tsx:148
- 根拠: 印は改名前の「今日の最後の位置の後の記録はほかの記録の行に出る」のまま、fixture は末尾に `kind:"other"`。改名後の「…記録なしの行に出る」の印は `crates/server/src/day_tests.rs:711` だけ。Global Constraints の「両方に印」に反する（Task 11 F1 と ⚠️ を確定）
- kind: technical
- 処置: fixed 11.1 —— fixture を末尾の記録なしの行にし、印を現在の名前にした（e8fcf80e）

## R4. 画面の位置の記録の題が spec の「場所 120 件」でなく「携帯端末の位置 120 件」になり、試験は偽の表示名「場所」で通している
- 成果物: web/src/__tests__/st25-open.test.tsx（Task 9 F1 / ⚠️）・web/src/DayView.tsx
- 根拠: spec の Scenario「位置の記録は 1 行にまとめる」の THEN は「場所 120 件」。本物の `core.source.display_name` は `携帯端末の位置`
- kind: conflict
- 処置: fixed D23 仮 —— 位置の記録の題は表示名に依らず「場所」（46755e13）。反転条件は D23

## R5. spec に無い名前の Scenario の印がある（check_scenarios の warn）
- 成果物: web/src/__tests__/st25-gaps.test.tsx:139
- 根拠: 印「『消した』の行の直後に『ほかの記録』の行がある」は spec に無い名前。`python3 scripts/check_scenarios.py . st25-day-timeline` が warn を出す
- kind: technical
- 処置: fixed 11.1 —— ただのコメントにした（5558351f。check_scenarios の st25 の warn 0）

## R6. `browser_through` が利用者のブラウザ履歴の全行を毎回読む
- 成果物: crates/server/src/day.rs:538-545
- 根拠: `max(ingest_time)` を `c02-browser-history` の全行に対してとり、`event_by_source_time` は `ingest_time` を持たない
- kind: technical
- 処置: fixed 2.1 —— 10 万行で Seq Scan 21.5 / 23.4 ms → 索引 `(user_id, logical_source, ingest_time)` で 0.085 / 0.094 ms。移行 202610121040_event_by_user_source_ingest を足した（81780e8f）

## R7. `reasons()` が記録なしの行ごとに SQL を 5 本ずつ直列に投げ、`collection_started_on` を行ごとに読み直す
- 成果物: crates/server/src/day.rs:676
- 根拠: Task 5 F3（未対応のまま）
- kind: technical
- 処置: fixed 5.1 —— `collection_started_on` を `day()` で 1 回だけ読む（c2ffb295）

## R8. ブラウザ履歴が一度も届いていない日（`browser_through` が NULL）に「まだ届いていない」の行を出さない判断が design に無い
- 成果物: openspec/changes/st25-day-timeline/design.md（D10）
- 根拠: Task 4 F1
- kind: daily
- 処置: fixed D10 仮 —— NULL のときは行を出さないと反転条件を書いた（2b3ded32）

## R9. `host_of` が `about:blank` を "about"、`chrome://newtab` を "newtab" のドメインとして返し、サイトの上位 3 に混ざりうる
- 成果物: crates/server/src/day_records.rs:187
- 根拠: Task 1 F4（未対応）
- kind: technical
- 処置: fixed 1.2 —— http / https 以外は None（70517a1a。単体と DB の試験）

## R10. 前日 23:50 から続く PC の前景が、今日 00:00 始まりの行の使った時間に数えられない
- 成果物: crates/server/src/day.rs:666-667
- 根拠: 読む窓が最も早い行の日の頭から始まり、窓より前に始まった前景を読まない（Task 3 F3。未対応）
- kind: technical
- 処置: fixed D6 —— 読む窓を前の日の頭からにした（c2ffb295。`st25_usage_foreground_from_the_previous_day_counts_from_midnight`）

## R11. 離席・停止しか無い時間に、要約が空の「ほかの記録」の行ができうる
- 成果物: crates/server/src/day.rs:690
- 根拠: 行にする条件が `is_listed()`（`Background` 以外）
- kind: technical
- 処置: rejected: 空の行ではない —— `is_listed()` は `Background` 以外を全部通す（day_records.rs:103-105）ので、その行を開くと離席・停止の記録が出る。D19（仮）の「どの行にも入らない時間の記録は『ほかの記録』の行に出す」のとおり

## R12. 細部の未対応 —— `records()` の `from >= to` が黙って空、合計（round）と上位（floor）の丸めの違い、`FoldedRow` / `RecordsView` の色の直書き、`GapRow` の key の重なり
- 成果物: crates/server/src/day_records.rs（Task 2 F2）・web/src/timeline.ts:131/138（Task 8 F4）・web/src/DayView.tsx（Task 9 F3。`gap-${kind}-${start}`）
- 根拠: final reviewer M8。2 台の PC が同じ時刻に離席すると React の key が重なる
- kind: technical
- 処置: fixed 2.1 —— (a) `from >= to` は 400（28c714c0）(b) 合計も切り捨て（9cce10be・e21502bb）(c) `paint(c, "detail" / "folded")`（9cce10be）(d) gap に `device_id`、鍵に端末と終わり（e21502bb）

## R13. 試験と計画の穴 —— seed の冪等を検証コマンドが判定していない・e2e の題名の検出が seed の形に依存し、畳んだ件数と 1,000 件の前提を確かめていない・C10 は明るい側だけ・見出しの確かめが `toContain("日")`・ルートの Scenario は jsdom だけ
- 成果物: tools/seed.sh（Task 6 F2 / ⚠️）・web/e2e/day-timeline.spec.ts（Task 10 F2〜F4）・web/src/__tests__/st25-contrast.test.ts / st25-routes.test.tsx（Task 7 F2 / F3 / ⚠️）
- 根拠: final reviewer M9
- kind: technical
- 処置: fixed 10.1 —— (a) seed を 2 回で rc 0/0・件数不変を実測（c01-app-usage 440 / c01-location 1411 / c02-window 1497）(b)(c) e91f7f25。(d) は tasks 7.1 の計画の判断（ルートは VT）のまま

## R14. 文書 —— `review/handoff.md` の「読んだ日時」が 2026-10-02 のまま・`deep.md` の Q8 の節が Q1〜Q7 の並びの外
- 成果物: openspec/changes/st25-day-timeline/review/handoff.md・deep.md
- 根拠: Task 11 F2 / F3
- kind: technical
- 処置: fixed 11.3 —— 5d70fa96

## R15. ST28 の e2e（`login.spec.ts` / `offsite.spec.ts`）の開き先を `/#/coverage` に直しているのに、それが PR に書かれていない
- 成果物: web/e2e/login.spec.ts・web/e2e/offsite.spec.ts（27cf5dc4）
- 根拠: ルート（`#/` が 1 日の画面）の変更に伴う直しで、ほかの Story のファイルに触れている
- kind: technical
- 処置: fixed 11.3 —— PR 本文（`docs/briefs/ST25-pr.md`）の「他 Story のファイルへの変更」に挙げる（finish）

## scoped re-review（71f2d9e..5d70fa9）: R1〜R15 は全件 ADDRESSED、新たな Critical / Important なし。残りの Minor 3 件を裁定する（fix は 1 回だけ）

## R16. 明示の `from` / `to` に長さの上限が無く、数か月の範囲を渡すと全期間を正規化する
- 成果物: crates/server/src/day_records.rs（records）・crates/server/src/lib.rs（/day/records）
- 根拠: R2 の直しで日の端で切らなくなった（scoped re-review）
- kind: technical
- 処置: rejected: crates/server/src/day_records.rs:609-610 は明示の範囲をそのまま読むが、認証の後ろの単一利用者の API で、画面が渡すのは行の範囲（滞在の長さ）だけ。読む量は自分の記録に限られ、他人に負荷を掛ける経路が無い。上限を足すなら D5 の 40 日が目安

## R17. 要約・事情の行間 1.45 を tokens.ts の外の局所の定数で持ち、ui-direction の下限ちょうどに張り付いている
- 成果物: web/src/DayView.tsx:49（`SUMMARY_LINE`）
- 根拠: scoped re-review。`docs/ui-direction.md:327`（UIR-25）の下限が 1.45、spec の閉じた行 200 px 以下・360 px で 6 行（spec.md:495-496）を満たすための値
- kind: technical
- 処置: rejected: web/src/DayView.tsx:49 の 1.45 は docs/ui-direction.md:327 の下限を割っておらず、本文の表面（15.5px / 1.70・色・角・密度）は変えていない。main 自身も caption に局所の行間を当てている（CoverageGrid.tsx・AchievementPanel.tsx）。下限を上げると e2e（200 px / 6 行）が落ちるので、変えたときは機械が止める

## R18. R10 の直しで `GET /day` が常に 2 日分を読み、design の Risks の「数十 ms」の見積もりに含まれていない
- 成果物: crates/server/src/day.rs:1055 付近
- 根拠: scoped re-review
- kind: technical
- 処置: rejected: 読みは D3 の日付で先に絞る SQL と `event_by_source_time` の索引に乗るので、1 日分が 2 日分になるだけで全件は読まない。e2e（PC 1,000 件以上の日）で `/day` を開く試験が 49 本とも通っている

## code-verify（HEAD e9d3a0bd）

席: code-verify（申告と実態のずれ）。最後の証跡（evidence.jsonl）は 1ec0a53e で、その後の 12 commit（28c714c0〜e9d3a0bd）は
サーバと画面のコードを変えている。**全項目の検証コマンドを HEAD で走らせ直した。**

申告: **30/30**（`- [x]` 30・`- [ ]` 0）。

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD） | 一致 |
|---|---|---|---|
| 1.1 | `story_deps.py included ST25 ST22` ・ `test -f deletion.rs && grep "erased" stays.ts` ・ `CT st25_allow_registry_covers_archive` | rc=0 / rc=0 / 1 本 passed | 一致 |
| 1.2〜1.4 | `CT st25_allow_` / `st25_fold_` / `st25_range_` | 18 / 5 / 4 本（`-- --list`）、`cargo test st25_` 全体で 88 passed・0 failed | 一致 |
| 2.1 / 2.2 | `CT st25_api_records_`・`tools/check-openapi.sh` / `CT st25_api_unchanged_` | 6 本 / rc=0 / 1 本 | 一致 |
| 3.1〜3.4 | `CT st25_usage_` / `st25_summary_` / `st25_other_` / `st25_api_day_`・check-openapi | 7 / 12 / 4 / 3 本・rc=0 | 一致 |
| 4.1 / 4.2 | `CT st25_gaprow_` / `st25_browser_` | 12 / 6 本 | 一致 |
| 5.1 / 5.2 | `CT st25_reasons_` / `st25_nostay_` | 8 / 1 本 | 一致 |
| **6.1** | `stack.sh up` → `seed.sh normal` ×2 → `curl $BIND/day?date=2026-09-07 \| jq -e …` | **rc=22**（R20）。口を 19050 に直すと seed 2 回とも rc=0・jq true | **不一致** |
| 7.1 | `VT st25-routes.test.tsx`・`! grep "Scenario: 稼働状況の画面の入口は変わらない"` | 9 passed / rc=0 | 一致 |
| 7.2 | `npx vitest run`・`! grep "api/stays?date" DayView.tsx` | 30 files・235 passed / rc=0 | 一致 |
| 7.3 | `grep "#/coverage" docs/screens.md`・`ET stack.spec.ts`・`ET coverage-year.spec.ts` | rc=0 / e2e 全体の中で通過 | 一致 |
| 8.1〜8.3 | `VT st25-summary` / `st25-gaps` / `st25-contrast` | 13 / 25 / 1 passed | 一致 |
| 9.1 / 9.2 | `VT st25-open` / `st25-fold` | 8 / 4 passed | 一致 |
| 10.1〜10.4 | `ET day-timeline.spec.ts`・`grep -c "^test(" … \| xargs test 14 -le` | e2e 49 passed（`WEB_PORT=5925 CI=1 STACK_RESET=1`。5180 は ashiato2-integ の vite が持っていて、既定のままだと別の画面へログインして 401）/ 14 | 一致（ただし R19） |
| 11.1 | `check_scenarios.py` / `review_triage.py` / `check_chain.py` | rc=0 / rc=0 / rc=0（この change の 86 Scenario はすべて印あり） | 一致 |
| 11.2 | fmt / clippy -D warnings / `cargo test --workspace` / `npm run lint && test && build` / check-openapi / `tools/smoke.sh` | rc=0 / rc=0 / 639 passed / rc=0・235 passed・rc=0 / rc=0 / 「縦串 OK」 | 一致 |
| 11.3 | `test -s review/handoff.md && grep $(git log -1 --format=%h -- docs/handoff)` | rc=0 | 一致 |

手 1（固定値の再計算）: 該当なし。確かめた範囲: (a) `TIMELINE_PAIRS` の 6 組をトークンの HSL から python（`colorsys`・WCAG の式・8 bit に丸め）で計算し直した ——
ライト 6.88 / 7.44 / 12.41 / 6.31、ダーク 6.58 / 8.31 / 8.03 / 5.09。どれも 4.5 以上で、`toneContrast` の判定と一致する。
(b) `st25_api_unchanged_events_and_stays_shapes` の鍵の直書きを、変更前（11b7d1f4）の `docs/openapi.json` から取り直した ——
`/events` は `event_time,id,logical_source,origin,raw,tz_id`、`/stays` は `criteria,date,entries` で一致する（`stay_store.rs` は origin/main との差分 0）。

手 2（ガードを壊す）: 該当なし。確かめた範囲（複製した worktree で 1 か所ずつ壊して戻した）: `from >= to` の 400 を `>` にすると `st25_api_records_empty_range_is_400` が落ちる /
`fold_visits` を外すと `st25_fold_same_visit_from_two_pcs_is_one` が落ちる / spec の印を 1 つ消すと `check_scenarios.py` が `FAIL (担保なし 1 件)`・rc=1 /
ルートの既定を稼働状況に戻すと vitest が 5 本落ちる。

手 4（本人の決定）: Q1 の値は落ちる。`SUMMARY_TOP` 3→4（サーバ 4 本・画面 4 本が落ちる）、`AWAY_ROW_MIN_MINUTES` 10→9（サーバ 2 本・画面 2 本）、
`>=` → `>`（サーバ・画面とも「ちょうど 10 分」の 1 本）、画面の離席の絞り込みを外す（1 本）、上位の `slice` を外す（2 本）、
`PHONE_USE_CAP_MINUTES` 10→15（2 本）。Q2 の（4）・Q4 のルートは手 2 のとおり。値で落ちないのは D5（仮）の 40 日だけ（R24）。
Q6（消す口を足さない）は「しない」で、固定する試験は無い（Non-Goals のため指摘にしない）。

手 6（隙間）: この Story は読むだけで、書く・捨てる経路が無い（`stay_store.rs` は main との差分 0。D14）。「捨てたものは復元できない」型は無い。
見つかったのは表示の欠け（R21・R24）と、検証の台本が別の DB へ書きにいく経路（R20）。

## R19. 「閉じた行は 200 px を超えない」「開いてすぐ 6 行以上」は、spec の「いつもの日」の前提（ブラウザの記録がある）を欠いた偽データで緑になっている。ブラウザの訪問が入ると 204.3 px で落ちる
- 成果物: web/e2e/day-timeline.spec.ts:102・127 / tools/seed.sh:319-324 / web/src/DayView.tsx（要約の行）
- 根拠: spec.md:495「いつもの日（滞在 9 件・PC とブラウザとスマホの記録がある日）」。HEAD の偽データの 2026-09-07 は `seed.sh` が
  「ブラウザの訪問は入れない（external_id_kind が 'none'）」で、`/day` の entries のうち `summary.browser` を持つ行は **0**（stay 9・move 9・no-record 2）。
  e2e は PC の 1,000 件の前提だけを確かめ（:93）、ブラウザの前提は見ていない。
  (1) 複製の木で `/api/day` に滞在の行だけブラウザの要約（github.com 18・docs.rs 7・example.com 5）を足して測った: 全体が入る行 **6 → 4**、最も高い行 **166.3 → 204.3 px**。
  (2) ST25 の使い捨て DB で `c02-browser-history` の `external_id_kind` を `record`（ST08 の移行後の値）にして `seed.sh normal` を入れ直した（訪問 179 件、要約に出る行 12）:
  `npx playwright test e2e/day-timeline.spec.ts` → **13 passed / 1 failed**、`閉じた行はどれも 200 px 以下` が `Expected: <= 200 / Received: 204.296875`。
  **ST08 の移行が main に入った時点で、この e2e は main で赤になる**（feat/st08-browser-history は走っている）
- kind: technical
- 提案: e2e に PC と同じ形で前提を足す（ブラウザの要約を持つ行が 1 つ以上・滞在 9 件。満たさなければ前提の崩れとして落とす）。
  そのうえで、ブラウザの要約が入った行が 200 px に収まるよう行の組み方を直すか、spec の数値をどう扱うかを決める。6 行の側は余裕が 0（いまちょうど 6）
- 処置: fixed D15 仮 —— 反転条件どおり上限を 200 → 240 px に見直した（726ac79b。spec の Requirement と Scenario 名・design D15 の ★ 2026-10-12・tasks 10.x の Scenario 名）。`external_id_kind` を `record` にして `seed.sh normal` を入れた ST25 の DB で、滞在の行の最大は 223.3 px（要約の欄 249 px で PC・ブラウザが各 3 行に折り返す）。行間は UIR-25 の下限ちょうど（R17）、見出しと「長さ ・ 範囲」は ST22 の正典の Scenario なので「詰める」は採らなかった。6 行以上は変えていない。e2e は滞在 9 件・PC とスマホの要約を前提として落とし、ブラウザの要約が無い（ST08 の移行の前）ときは注記に残す。ブラウザ入りの DB で `day-timeline.spec.ts` 14 passed、作り直した DB（`CI=1 STACK_RESET=1`）で e2e 全体 49 passed

## R20. 6.1 の検証コマンドは Story の worktree で別のサーバを叩き、HEAD で rc=22 になる。`seed.sh` も同じ口へ書きにいく
- 成果物: openspec/changes/st25-day-timeline/tasks.md（6.1）/ tools/seed.sh:19 / tools/ports.sh
- 根拠: 6.1 は `. ./.env` の後に `curl "http://$BIND/day…"` を撃つ。`.env` の `BIND` は 127.0.0.1:18787、`stack.sh` は `tools/ports.sh` で
  この worktree の口（19000+2×25 = **19050**）に差し替えて立てる（`stack.sh:25`）。`seed.sh:19` の `BIND="${BIND:-…}"` は `.env` の 18787 を優先する。
  HEAD で tasks の本文どおりに走らせて **rc=22**（`seed.sh` の最初の `curl -f` で止まる）。このとき 127.0.0.1:18787 には ashiato2-integ のサーバ（pid 845459、cwd `/home/yosis/dev/ashiato2-integ`）が
  待ち受けていて `/events` が 401 だったので書き込まれなかった。**API_TOKEN が同じなら、ST25 の偽データ 1,937 件がほかの worktree の DB に入る。**
  `BIND=127.0.0.1:19050` を渡すと seed 2 回とも rc=0・`jq -e` が true（中身は合っている）。
  6.1 の PASS の証跡は 8fcde699（2026-10-02）だけで、`tools/ports.sh` の導入（430196c9、2026-10-05）より前
- kind: technical
- 提案: 6.1 の検証を `plan_fix.py` で直す（`. tools/ports.sh` の後の `127.0.0.1:$ASHIATO_HTTP_PORT` を撃ち、`seed.sh` にも同じ `BIND` を渡す）。
  `seed.sh` が `.env` の `BIND` を worktree の口より優先する順は ST05（ports.sh）の側の事なので `docs/handoff/` に書く
- 処置: fixed 6.1 —— deep.md Q9 の答え（plan_fix.py で直す）のとおり。旧コマンドを verify-run 6.1 で FAIL（rc=22。5180 は ashiato2-integ の確認バッチが持つので WEB_PORT=5230 を環境で渡した）と記録してから、`plan_fix.py` で入口を `tools/st25-seed-check.sh` に直した（`plan_fix.py` は新しい入口に repo の実行ファイルを求める。中身は原文と同じ手順で、`. tools/ports.sh` の後に `BIND=127.0.0.1:$ASHIATO_HTTP_PORT` を seed と curl に渡すだけ）。新しい入口で verify-run 6.1 PASS（rc=0）。記録は `plan-corrections.md`。`seed.sh` の口の順と WEB_PORT 5180 の取り合いは ST05 の ports の被らない割り当ての fix で拾う（Q9 の「効く先」）。scoped re-review: 判定は旧コマンドと逐語・終了コードの経路も問題なし。Minor 1 件（台本のコメントの因果と、初版の rc=143 の FAIL が記録に無い）→ コメントを直し、plan-corrections.md に補足を足して走らせ直し PASS

## R21. 日付をまたいだ「いま離席中」が、今日にも前の日にも行として出ない
- 成果物: crates/server/src/day_records.rs:415-430（`normalize_all` の `open` の条件）
- 根拠: 複製の木に探りの試験を足して実行した —— 2026-09-06 23:50 の `idle enter`（leave なし）、いま = 2026-09-07 00:30 で
  `GET /day?date=2026-09-07` の離席の行 **0**、`date=2026-09-06` も **0**。条件 `jst_date(row.event_time) == jst_date(now)` が、前の日に始まった離席を今日の側で落とす。
  spec.md:363 は「今日の画面では、いまの時刻より後を取れていない時間の行にしない」だけで、日をまたぐ場合を除いていない。
  FR-83 の本文（何も無い時間を「PC を使っていなかった」と読ませない）が、0 時を挟んで離席している間だけ崩れる。leave が届けば戻る（記録は失われない）
- kind: technical
- 提案: `open` を「leave が無く、enter が今日の終わりより前で、いまが enter より後」にし、今日の範囲は日の端で切る。
  前の日 23:50 の enter と 0:30 のいまで、今日に「離席 0:00 – 0:30」が出る試験を足す
- 処置: fixed 4.1 —— leave の無い enter は今日か前の日に始まったものをいままでの離席にし、行は日の端で切る（4221f2e2。`st25_gaprow_away_now_from_the_previous_day_is_a_row_on_both_days`: 前の日 23:50 の enter・いま 0:30 で、今日に 0:00 – 0:30、前の日に 23:50 – 24:00。単体の `st25_allow_row_open_enter_is_away_until_now_from_today_or_the_previous_day`）。古い条件に戻すと 2 本が落ちる。2 日前より前の enter は従来どおりいままでにしない（leave の届かなかった古い enter を毎日の離席にしない）

## R22. tasks.md の 3.3 と 8.1 が、Q7 で名前を変えた Scenario を古い名前のまま挙げている
- 成果物: openspec/changes/st25-day-timeline/tasks.md:86・152
- 根拠: `grep -rn "今日の最後の位置の後の記録はほかの記録の行に出る"` → tasks.md:86・152（と evidence.jsonl の古い警告）だけ。spec.md:272 の名前は「…記録なしの行に出る」で、
  試験の印（day_tests.rs・st25-summary.test.tsx）は新しい名前。3.3 の本文も「今日の最後の位置からいままで」に `kind: "other"` の行を足すと書き、Q7 の決定（足さない）と逆
- kind: technical
- 提案: `plan_fix.py` で 3.3・8.1 の Scenario 名と 3.3 の本文を Q7 に合わせる
- 処置: fixed 3.3 —— tasks 3.3・8.1 の Scenario 名を「…記録なしの行に出る」に、3.3 の本文を Q7（今日の最後の位置からいままでは記録なしの行に要約を添える）に揃えた（726ac79b）。検証コマンドは変えていない

## R23. 「行を開くとその時間の記録が時刻順に出る」の「アドレスは変わらない」を試験が観測していない
- 成果物: web/src/__tests__/st25-open.test.tsx:68
- 根拠: 試験は `expect(window.location.hash).not.toContain("records")` だけ。複製の木で行を開く処理（DayView.tsx:229）に
  `window.history.replaceState(null, "", "#/day/x/open")` を足しても `npx vitest run src/__tests__/st25-open.test.tsx` → **8 passed**
- kind: technical
- 提案: 開く前の `location.hash` を取っておき、開いた後に等しいことを確かめる
- 処置: fixed 9.1 —— 開く前の `location.hash` を取り、開いた後に等しいことを確かめる（c8ea9a1b）。DayView.tsx:229 に `replaceState(null, "", "#/day/x/open")` を足すと `st25-open.test.tsx` が 1 本落ちる

## R24. D5（仮）の 40 日を 1 日にしても全部の試験が通る
- 成果物: crates/server/src/day_records.rs:23（`RANGE_MARGIN_DAYS`）/ day_records_tests.rs（`st25_range_`）
- 根拠: 複製の木で 40 → 1 にして `cargo test -p ashiato-server st25_` → **88 passed / 0 failed**。区間の記録はスマホの取りこぼし（`gap`）が `event_time` を終わりに置き（`Span::BeginEnd`）、
  PC の停止は始まりに置く（`Span::EndKey`）ので、幅が足りないと日の外から続く区間が黙って消える。いまの試験の区間は最長でも前の日 23:40 からで、1 日の幅に収まる
- kind: technical
- 提案: 終わりが 2 日以上先のスマホの取りこぼしと、始まりが 2 日以上前の PC の停止がその日に出る試験を 1 本ずつ足し、`RANGE_MARGIN_DAYS` を名指しで固定する
- 処置: fixed 1.4 —— `RANGE_MARGIN_DAYS` を名指しで 40 と固定し、39 日前に始まる PC の停止と 39 日後に終わるスマホの取りこぼしがその日に出る試験を足した（4221f2e2。`st25_range_long_intervals_within_the_margin_are_returned`）。2 日にすると落ちる

## R25. 画面の文字を言う Scenario のうち 6 本が、サーバの試験（JSON）だけで担保されている
- 成果物: crates/server/src/day_tests.rs / web/src/__tests__/st25-summary.test.tsx
- 根拠: 印の置き場を全 86 本について grep した。「離席の時間はアプリを使った時間に数えない」「10 分に満たない離席も使った時間から差し引く」
  「スマホの使った時間は 1 回 10 分で打ち切る」「同じ時間のアプリは名前の順に並ぶ」「ドメインは www を外して小文字で束ねる」「日付をまたぐ行の要約は両方の日で同じ」は
  THEN が「その行の要約に…出る」（画面の文字）なのに、印は `day_tests.rs` だけ。Global Constraints は「画面の文字を言う Scenario は、サーバの試験と jsdom の両方に印」。
  tasks 8.1 の Scenario の一覧がこの 6 本を落としていて、計画の段で抜けた
- kind: technical
- 提案: st25-summary.test.tsx に、サーバの試験と同じ値の `/api/day` を描いて「VS Code 25 分」「Mail · Slack の順」「example.com 2」を確かめる試験を足し、印を置く
- 処置: fixed 8.1 —— 6 本の Scenario に、サーバの試験と同じ値の `/api/day` を描いて画面の文字を確かめる jsdom の試験と印を足した（c8ea9a1b。「VS Code 0:25」「VS Code 0:24」「地図 0:10」「Mail 0:20 · Slack 0:20」「example.com 2」・日をまたぐ滞在を両方の日で描いて同じ文字）。tasks 8.1 の Scenario の一覧にも足した（726ac79b）

## scoped re-review（e9d3a0b..726ac79）: R19・R21〜R25 は ADDRESSED、R20 は deep.md Q9 に積んだ（escalated）。新たな Critical / Important なし

残りの Minor は 2 件。tasks 10.1 の本文に「200 以下」が残っていた件は、R19 の直しの書き換え漏れなので、同じ直しの中で 240 に揃えた。
design D15 の「200 px は ST16 の実測…」の件は、直後の ★ 2026-10-12 の注記で経緯として読めるので残した（指摘にしない）。
