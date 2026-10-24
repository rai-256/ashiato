# final review — st41-event-end-time

## final review（f0c6cf1..6a8f3c5）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md` をそのまま）。review package
`.superpowers/sdd/tasks/review-f0c6cf14..6a8f3c51.diff`（32 commit）と、全 Task の `task-*-findings.md` の Minor と ⚠️、
Task 5 の ledger の parked（F1）を見た。`git merge-tree origin/main HEAD` で main との衝突も試した。
判定: **With fixes**（Critical 0 / Important 3 / Minor 10）。

reviewer が閉じた ⚠️: Task 5（access_log は `MatchedPath` だけで query を残さない。`parse_period` は値を本文に返さない）・
7.1b（`stay_store.rs`・`collector-android`・`crates/collector-windows` は差分なし）・2 回当ての試験の deadlock（c52e0333 で使い捨ての DB へ）。

## R1. main（CH02 の窓の形）と `export/excluded.rs` が衝突し、D11 がまだ main の `overlap_sql` に当たっていない
- 成果物: crates/server/src/export/excluded.rs
- 根拠: `git merge-tree --write-tree origin/main HEAD` → `CONFLICT (content): Merge conflict in crates/server/src/export/excluded.rs`（衝突はこの 1 ファイルだけ）。ブランチは CH02 前の `overlap_sql(rows, next_after, …)` に列を足したが、main は `overlap_sql(rows, is_event, …)` と `span_next` の窓の形で、前の版の副問い合わせも `v.id, e.logical_source, v.event_time, v.payload, v.raw` に絞った（cac51b8b）。D11 は「どちらが先に main に入っても、ST41 の下流は main にある `overlap_sql` に足す」。解き違えると申し送り C7 の `loss: exported` が戻る。merge_gate は origin/main へ rebase するので、ここで解かないと gate が衝突で止まる
- kind: technical
- 処置: fixed D11 —— origin/main 6bb4584e へ rebase し、main の `overlap_sql(rows, is_event, …)` と絞った副問い合わせに終わりの列を足した。scoped re-review で衝突なし・C7 の順を確かめた

## R2. 主張・主観・場所（authored）の錠が終わりの 3 列を凍結していない（Task 1 の ⚠️ / F1）
- 成果物: migrations/202610231200_event_end_time.sql, tools/check-immutable.sh
- 根拠: `202609160220_personal_attributes.sql:94` `reject_claim_rewrite`・`202610141000_subjective_log.sql:75` `reject_feeling_rewrite`・`202610020030_places.sql:56` `reject_place_record_rewrite` は `event_time`・`tz_offset_min`・`tz_id` を凍結し `end_*` を名指ししない / `require_version_or_ledger` は `origin <> 'collected'` で素通し / `tools/check-immutable.sh:882` が 3 列を `open_cols` に入れたので点呼も通る。履歴なしで authored の行の終わりを書き換えられ、書き出しと `/events` にそのまま出る。D5 は collected だけを範囲にし、authored は上流で決まっていない。凍結する側は後で緩めても何も失わない
- kind: conflict
- 処置: fixed D19 仮

## R3. design D10 / premises.md の「点の記録は既存の時刻の索引で引ける」は誤り
- 成果物: openspec/changes/st41-event-end-time/design.md, openspec/changes/st41-event-end-time/premises.md
- 根拠: `GET /events` は `user_id` で絞らず、`event_time` を先頭に持つ索引は無い（`(logical_source, event_time)` と `(user_id, event_time) WHERE deleted_at IS NULL` だけ。PG17 に skip scan は無い）。`source` なしの期間の求めの点の側（`end_time IS NULL AND event_time >= $1 AND event_time < $2`）は全走査。正しさの退行ではない（期間なしの今の挙動も全行）
- kind: technical
- 処置: fixed D10

## R4. 期間で引くときの点の側の索引（または user で絞る）を決める
- 成果物: crates/server/src/lib.rs（`events` の UNION の後半）
- 根拠: R3。期間を引く最初の衛星の読み出しは ST45。索引を足すか user で絞るかは、その読み出しの形で決まる
- kind: defer
- 処置: deferred ST45

## R5. `from` / `to` の `+09:00` が query で空白に戻り 400 になる。openapi に形式の説明も試験も無い（M1）
- 成果物: crates/server/src/lib.rs（`parse_period`）, docs/openapi.json
- 根拠: 通るのは `Z` か `%2B09:00`。`Z` 以外のずれを試す試験が無い
- kind: technical
- 処置: fixed D10 —— `parse_period` の doc・openapi の説明・`st41_range_offset_period_in_query_string`

## R6. Task 7 F1: `end_time_tests.rs` の `versions_with_end == 0` は常に成り立つ空のアサート（M3）
- 成果物: crates/server/src/end_time_tests.rs:1818 付近
- 根拠: `rebuild_all` が 1 回だけなので版は 0 行
- kind: technical
- 処置: fixed 7.1 —— `st41_stays_and_collected_rows_have_no_end_column` が版を 1 行以上作ってから 0 件を見る

## R7. Task 7 F2: CHECK の数え方が `conname LIKE '%\_end\_%'` に依る（M4）
- 成果物: crates/server/src/end_time_tests.rs
- 根拠: 似た名前の制約が増えると数がずれる。4 つの名前を列挙する
- kind: technical
- 処置: fixed 7.1 —— CHECK を 4 つの名前で列挙する

## R8. Task 4 F1: `st41_coverage_started_on_uses_start` が「終わりの UTC の日」で決める誤った実装でも通る（M5）
- 成果物: crates/server/src/（`st41_coverage_started_on_uses_start`）
- 根拠: 終わり 22:00Z が始まりと UTC で同じ日
- kind: technical
- 処置: fixed D12 仮（終わりを 10-02T03:00Z にし、終わりの日で決める実装が落ちる形に）

## R9. Task 2 F1: `truncate_end_to_micros` を直接見る単体試験が無い（M6）
- 成果物: crates/server/src/ingest.rs
- 根拠: `st41_update_sub_micro_end_change_is_not_an_update` が間接に固定しているだけ
- kind: technical
- 処置: fixed D18 仮（`st41_ingest_truncate_end_to_micros`）

## R10. Task 1 F3: `check-immutable.d/` は古い門の上で走ることが README に無い（M7）
- 成果物: tools/check-immutable.d/README.md
- 根拠: `.d/` は本体の戻し手順の節の後に読まれる。ST41 は schema を作り直して避けたが、次の Story が同じ罠に落ちる
- kind: technical
- 処置: fixed D15 —— `tools/check-immutable.d/README.md`

## R11. Task 6 F5: 消した滞在の前の版の `coalesce(v.end_time, …)` を通る試験が無い（M8）
- 成果物: crates/server/src/export/excluded.rs:203 付近
- 根拠: 列を外しても落ちる試験が無い
- kind: technical
- 処置: fixed D11 —— `st41_export_erased_stay_version_end_column_is_read`

## R12. Task 5 F1（parked）: `day_records_tests.rs` の `st25_api_unchanged_events_and_stays_shapes` の `.filter`（M2）
- 成果物: crates/server/src/day_records_tests.rs:1051
- 根拠: 期待値の鍵 6 つは不変。`export_end_time_tests` が共有 DB に `c01-clock` の終わりつきの生きた行を置くので、元の assert は原理的に成り立たない。裁定（人間が受け入れ 2026-10-24）は妥当。`rebuild_all` の `s01-stay` の行の鍵を見なくなった点は残る
- kind: technical
- 処置: rejected: `crates/server/src/day_records_tests.rs:1051` の期待値の鍵 6 つは不変。`crates/server/src/export_end_time_tests.rs` が共有 DB に `c01-clock` の終わりつきの生きた行を置くので元の assert は原理的に成り立たない。人間が受け入れた裁定（2026-10-24）のまま park（scoped re-review も妥当と判定）

## R13. 計画外の変更と証跡の見え方を PR 本文に残す（M9・M10）
- 成果物: docs/briefs/ST41-pr.md
- 根拠: 過去の移行 `202609120943` の編集（`to_regclass` で囲む）・`check-openapi.sh` の除外（Task 2 F2）・`export_ledger_tests.rs` の `.last()` → `.any()`・`check-immutable.sh` の戻し手順の節への挿入。7.2 の最新の証跡は `check_scenarios.py` だけで、全体の `cargo test`・clippy・smoke は report の申告
- kind: technical
- 処置:

## scoped re-review（4071c1d..878d10a、R1 は origin/main との差）

席: SDD の `re-review-prompt.md`。判定: **All findings addressed, no new Critical/Important breakage**。
R1〜R3・R5〜R11 は ADDRESSED、R4・R12・R13 はコードでは直さない（R4 は ST45 へ、R12 は裁定のまま、R13 は finish の PR 本文で閉じる）。
R2 の錠の写しは main 上の唯一の定義（後の移行での作り直し無し）と、足した 8 行を除いて同じ。`.down.sql` は元と完全に同じ。

残り（ruling つきで park。指摘にはしない）:
- Minor: `tools/check-immutable.sh:893-896` の `frozen_cols` 3 列が同じ UPDATE を 3 回打つ（結果は正しい）
- Minor: `.superpowers/sdd/tasks/final-fix-report.md` の verify-run の表の 7.2 が「(7.2 の結果)」のまま
- 878d10a0 の全体の `cargo test -p ashiato-server` の証跡は rc=101（1450 passed / 2 failed: `archive_scan_interval_change_takes_effect_without_restart`・
  `stay_rebuild_chain_of_short_moves_settles`、ともに時間に依る試験で単独では通る）。Ruling: fix の差分と無関係の負荷依存の揺れとみる ——
  全体で通った証跡は verify-run（7.2）の取り直しで得る —— 外れていればグラフの verify が止める
- 取り込み口の更新の経路（`lib.rs:1711`）で authored の行に終わりだけ違う版が届くと錠が断り 500。始まりの列でも既にある振る舞いで、新しい穴ではない
