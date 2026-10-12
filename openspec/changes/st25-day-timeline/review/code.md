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
- 処置:

## R2. 日付をまたぐ行で、要約（行の実際の範囲）と開いた中身（その日の端で切る）が食い違う
- 成果物: crates/server/src/day_records.rs:600-602・web/src/DayView.tsx:272
- 根拠: `records()` が明示の `from` / `to` も `day_bounds(day)` に収める。design D7「行の範囲は**行の実際の始まりから終わりまで**」・spec「行を開くと、その行の時間に入る…記録を出す」（spec.md:284）と矛盾。
  例: 09-07 23:00〜09-08 02:00 の滞在は、09-08 の要約に 23:30 の Code が入るが、開くと 00:00 以後しか出ない。開いた中身を固定する試験が無い
- kind: technical
- 処置:

## R3. Q7 で名前を変えた Scenario の jsdom 側の印が無く、古い振る舞い（末尾に「ほかの記録」の行）を固定した試験が残っている
- 成果物: web/src/__tests__/st25-summary.test.tsx:148
- 根拠: 印は改名前の「今日の最後の位置の後の記録はほかの記録の行に出る」のまま、fixture は末尾に `kind:"other"`。改名後の「…記録なしの行に出る」の印は `crates/server/src/day_tests.rs:711` だけ。Global Constraints の「両方に印」に反する（Task 11 F1 と ⚠️ を確定）
- kind: technical
- 処置:

## R4. 画面の位置の記録の題が spec の「場所 120 件」でなく「携帯端末の位置 120 件」になり、試験は偽の表示名「場所」で通している
- 成果物: web/src/__tests__/st25-open.test.tsx（Task 9 F1 / ⚠️）・web/src/DayView.tsx
- 根拠: spec の Scenario「位置の記録は 1 行にまとめる」の THEN は「場所 120 件」。本物の `core.source.display_name` は `携帯端末の位置`
- kind: conflict
- 処置:

## R5. spec に無い名前の Scenario の印がある（check_scenarios の warn）
- 成果物: web/src/__tests__/st25-gaps.test.tsx:139
- 根拠: 印「『消した』の行の直後に『ほかの記録』の行がある」は spec に無い名前。`python3 scripts/check_scenarios.py . st25-day-timeline` が warn を出す
- kind: technical
- 処置:

## R6. `browser_through` が利用者のブラウザ履歴の全行を毎回読む
- 成果物: crates/server/src/day.rs:538-545
- 根拠: `max(ingest_time)` を `c02-browser-history` の全行に対してとり、`event_by_source_time` は `ingest_time` を持たない
- kind: technical
- 処置:

## R7. `reasons()` が記録なしの行ごとに SQL を 5 本ずつ直列に投げ、`collection_started_on` を行ごとに読み直す
- 成果物: crates/server/src/day.rs:676
- 根拠: Task 5 F3（未対応のまま）
- kind: technical
- 処置:

## R8. ブラウザ履歴が一度も届いていない日（`browser_through` が NULL）に「まだ届いていない」の行を出さない判断が design に無い
- 成果物: openspec/changes/st25-day-timeline/design.md（D10）
- 根拠: Task 4 F1
- kind: daily
- 処置:

## R9. `host_of` が `about:blank` を "about"、`chrome://newtab` を "newtab" のドメインとして返し、サイトの上位 3 に混ざりうる
- 成果物: crates/server/src/day_records.rs:187
- 根拠: Task 1 F4（未対応）
- kind: technical
- 処置:

## R10. 前日 23:50 から続く PC の前景が、今日 00:00 始まりの行の使った時間に数えられない
- 成果物: crates/server/src/day.rs:666-667
- 根拠: 読む窓が最も早い行の日の頭から始まり、窓より前に始まった前景を読まない（Task 3 F3。未対応）
- kind: technical
- 処置:

## R11. 離席・停止しか無い時間に、要約が空の「ほかの記録」の行ができうる
- 成果物: crates/server/src/day.rs:690
- 根拠: 行にする条件が `is_listed()`（`Background` 以外）
- kind: technical
- 処置: rejected: 空の行ではない —— `is_listed()` は `Background` 以外を全部通す（day_records.rs:103-105）ので、その行を開くと離席・停止の記録が出る。D19（仮）の「どの行にも入らない時間の記録は『ほかの記録』の行に出す」のとおり

## R12. 細部の未対応 —— `records()` の `from >= to` が黙って空、合計（round）と上位（floor）の丸めの違い、`FoldedRow` / `RecordsView` の色の直書き、`GapRow` の key の重なり
- 成果物: crates/server/src/day_records.rs（Task 2 F2）・web/src/timeline.ts:131/138（Task 8 F4）・web/src/DayView.tsx（Task 9 F3。`gap-${kind}-${start}`）
- 根拠: final reviewer M8。2 台の PC が同じ時刻に離席すると React の key が重なる
- kind: technical
- 処置:

## R13. 試験と計画の穴 —— seed の冪等を検証コマンドが判定していない・e2e の題名の検出が seed の形に依存し、畳んだ件数と 1,000 件の前提を確かめていない・C10 は明るい側だけ・見出しの確かめが `toContain("日")`・ルートの Scenario は jsdom だけ
- 成果物: tools/seed.sh（Task 6 F2 / ⚠️）・web/e2e/day-timeline.spec.ts（Task 10 F2〜F4）・web/src/__tests__/st25-contrast.test.ts / st25-routes.test.tsx（Task 7 F2 / F3 / ⚠️）
- 根拠: final reviewer M9
- kind: technical
- 処置:

## R14. 文書 —— `review/handoff.md` の「読んだ日時」が 2026-10-02 のまま・`deep.md` の Q8 の節が Q1〜Q7 の並びの外
- 成果物: openspec/changes/st25-day-timeline/review/handoff.md・deep.md
- 根拠: Task 11 F2 / F3
- kind: technical
- 処置:

## R15. ST28 の e2e（`login.spec.ts` / `offsite.spec.ts`）の開き先を `/#/coverage` に直しているのに、それが PR に書かれていない
- 成果物: web/e2e/login.spec.ts・web/e2e/offsite.spec.ts（27cf5dc4）
- 根拠: ルート（`#/` が 1 日の画面）の変更に伴う直しで、ほかの Story のファイルに触れている
- kind: technical
- 処置:
