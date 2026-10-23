# CH02 コードレビュー

## final review（536d22c..7e8332a）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`、review package `.superpowers/sdd/tasks/review-536d22c8..7e8332ad.diff`）。
判定: **Ready to merge: Yes**（Critical 0・Important 0・Minor 4）。fix は 1 回（9485ef39）、scoped re-review 1 回 → 3 件とも ADDRESSED。
再レビューが fix の中に Minor 1 件（足した doc の「だけ」が言い過ぎ）を挙げたので、その語だけ直した（2ad98ccb。下の R1 の処置に含める）。

前の Task で確かめられなかった項目（⚠️）の確かめ:
- Task 5（`-- --list` で 1 本以上）: `cargo test -p ashiato-server ch02_ -- --list` で 19 本（6.1 の下限 18 以上）
- Task 4（`ctes()` の枠の別名）: 本体は `ORDER BY q.event_time DESC` の後に改行して `RANGE …`。試験は空白を詰めて比べるので一致
- Task 3（枠を `CURRENT ROW` にした変異で RED）: 実行はせず読んで確かめた。同じ時刻の仲間が枠に入り幅 0 になるので、同じ時刻の組の試験・前の版のずれ 0 の族・4.4 の枠の固定が落ちる
- Task 6（ST09 のコードの上での `export_`）: この木では確かめられない。確認バッチの統合の木で `CT st09_exp_` / `CT export_` を見る（tasks.md 6.2・plan-corrections.md の承認どおり）
- `check_scenarios.py . ch02-export-next-record-window` rc=0

前の Task の Minor で処置しないもの（reviewer の判定）: Task 1 F3（Task 2・3 の試験で通る）、Task 2 F2（族「端末 NULL」の a が見分ける）、
Task 3 F1（`ch02_span_record_without_coordinates_ends_the_width` が見分ける）、Task 3 F2（日付は 2000-01-01 固定）、Task 3 F3（既存の試験と同じ形。この change の外）。

PR 本文に書くこと（reviewer の勧め）: (a) ST09 の上での試験は確認バッチの統合の木に任せた、(b) 120 万行・消した滞在ありの 1 通りで件数の数え（summary_seconds）が 25.5 秒と 3 秒の線を越えた（ST33 D13 の件数の出し方を替えるかは本人）、
(c) 意図した挙動の変更は同じ時刻の記録の先のほうが除かれる側に変わる 1 点だけ。Task 6 F3（`/tmp/ashiato-export-*` の漏れ）はこの change の持ち込みではない。

## R1. `overlap_sql` の doc が、行 `r` が `user_id` `device_id` を持つ前提のまま（組の鍵はいま `span_next` が持つ）
- 成果物: crates/server/src/export/excluded.rs
- 根拠: excluded.rs:252（7e8332a）の doc。`overlap_sql` の本体は `r` から `id` `logical_source` `event_time` `payload` `raw` だけを読む。Task 2 F1 の未処置
- kind: technical
- 処置: fixed 2.2 （doc を直した 9485ef39。再レビューの指摘で JOIN の理由に `logical_source` を親から取ることを足した 2ad98ccb）

## R2. `scale_reads` の doc が「`legacy` が偽なら」だが、引数名は `legacy_too`
- 成果物: crates/server/src/export_window_tests.rs
- 根拠: export_window_tests.rs:1037（7e8332a）。Task 4 F2 の未処置
- kind: technical
- 処置: fixed 4.2 （9485ef39）

## R3. 量の試験のコメント「消した滞在（01:00–02:00）が 1 件ある」が、`ASHIATO_EXPORT_VOLUME_ERASED=0` のときに成り立たない
- 成果物: crates/server/src/export_file_tests.rs
- 根拠: export_file_tests.rs:1458（7e8332a）。直前の doc は既定 `1`・`0` なら置かないと書き、本体は `if erased` で分ける。Task 5 F2 の未処置
- kind: technical
- 処置: fixed 5.1 （9485ef39。「既定では…」にした）

## R4. 線形の試験の `bulk_windows` は端末を記録ごとに変えるので組の大きさが 1。1 組に多くの記録がある形での窓のソートの重さは測っていない
- 成果物: crates/server/src/export_window_tests.rs
- 根拠: `bulk_windows` の配置（plan 4.2 の指定どおり）
- kind: technical
- 処置: rejected: 物差しは読んだ行の数（design.md の D7）で、組の大きさに依らない（plan の tasks.md:85 の 4.2 が配置をこの形に指定）。1 組に多くの記録がある実データの重さは 5.2 の実測（本物の写し 約 120 万行・146.5 秒。design の表 6 通り）が代わりに見ている。reviewer も「直さなくてよい」
