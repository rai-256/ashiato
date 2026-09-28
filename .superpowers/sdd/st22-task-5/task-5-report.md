# Task 5 実装レポート

## 実装内容

- `stay_store::EntryKind` に `Erased` を追加した。
- `core.stay_erased` ビューから当日と重なる削除区間を読み、`rebuild:absorbed` を除外した。
- 重なる・隣り合う削除区間を結合し、本人が消した滞在の識別子だけを `stay_ids` に載せる `DayEntry` を返すようにした。
- 削除区間を no-record の判定前に差し引き、残った生の点間隔を gap 閾値で再判定した。移動の被覆にも削除行を含め、削除時間が no-record / move と重ならないようにした。
- 既存の day-view テストを `stay_day_view` フィルタで実行できる名前に整え、Scenario 印を追加・更新した。

## TDD Evidence

### RED

追加した `day_view_erased_merges_adjacent_ranges_with_restorable_ids` と
`day_view_erased_not_no_record_or_move` を実装前に実行した。

```text
cargo test -p ashiato-server day_view_erased
test result: FAILED. 0 passed; 2 failed
隣り合う削除区間が 1 行でない
10:00 – 11:00 の消した行が無い
```

削除区間を `erased` 行として生成する実装がまだ無かったための失敗である。

### GREEN

```text
cargo test -p ashiato-server day_view_erased
test result: ok. 4 passed; 0 failed

cargo test -p ashiato-server day_view_erased_not_no_record
test result: ok. 1 passed; 0 failed

cargo test -p ashiato-server stay_day_view
test result: ok. 9 passed; 0 failed
```

## 検証

`scripts/verify-run` の証跡を `openspec/changes/st22-record-deletion/evidence.jsonl` に追加した。

- 5.1: PASS。`day_view_erased` 4 本。
- 5.2: PASS。`day_view_erased_not_no_record` 1 本。
- 5.3: PASS。`stay_day_view` 9 本。
- `cargo test --workspace -- --test-threads=1`: 352 passed, 0 failed。

Web の指定 `cd web && npm run test -- DayView` は、依存導入後も `No test files found` (rc=1) になった。実在するテストファイルは `day-view*.test.tsx` であり、フィルタの大文字小文字が一致しない。さらに `verify-run 5.3 --command` はこの第2コマンドを本文の検証コマンドとして認識しなかったため、tasks.md は変更していない。

`check_scenarios.py . st22-record-deletion` は、Task 6/8 など下流で担保する14 Scenario が未印のため FAIL になった。Task 5 の担当 Scenario に関する Rust 印と検証は上記のとおりである。

## 変更ファイル

- `crates/server/src/stay_store.rs`
- `crates/server/src/stay_tests.rs`
- `openspec/changes/st22-record-deletion/evidence.jsonl`
- `.superpowers/sdd/st22-task-5/task-5-report.md`

## Self-review

- 削除済み行の読み出しは `core.stay_erased` ビュー経由に限定した。
- `rebuild:absorbed` は行にも `stay_ids` にも含めない。
- no-record の判定は、既存の日をまたぐ端の挙動を壊さないよう、生の点間隔で閾値判定してから表示日の端へ切っている。
- 未解決の実装上の懸念はない。Web の検証コマンドの不成立は plan 側の既存問題として上記に記録した。

## Fix report (review 1)

- F1/F2 の原因を確認した。指定された `npm run test -- DayView` は、実在する `day-view*.test.tsx` と大小文字が一致せず、対象 0 件で rc=1 になっていた。
- ST16 のテスト本文と Scenario 印を変更せず、`day-view.test.tsx` と `day-view-limits.test.tsx` を `DayView.test.tsx` と `DayView-limits.test.tsx` に改名し、指定フィルタで実行可能にした。
- RED: `scripts/quiet-run web-dayview-exact -- bash -lc 'cd web && npm run test -- DayView'` — rc=1、`No test files found`。
- GREEN: `scripts/verify-run 5.3` — Rust/Web とも PASS。Web の `cd web && npm run test -- DayView` は rc=0。
- 回帰確認: `scripts/quiet-run web-full -- bash -lc 'cd web && npm run test'` — rc=0、20/20 test files、135/135 tests passed。
- `openspec/changes/st22-record-deletion/evidence.jsonl` に現行ツリーの 5.3 PASS 証跡を追加した。

## Fix report (review 2)

- F1/F2 の対象である Web 検証を現行ツリーで再実行し、指定コマンドが対象テストを選択できることを確認した。
- `scripts/quiet-run web-dayview-current -- bash -lc 'cd web && npm run test -- DayView'` — rc=0。
- `scripts/verify-run 5.3` — Rust の `stay_day_view` 9 本が PASS、Web の `cd web && npm run test -- DayView` が PASS。現行ツリーの証跡を `evidence.jsonl` に追記した。

## Fix report (review 3)

- F1/F2 について、現行ツリーの指定 Web 検証を再実行し、対象テストが選択されることを確認した。
- `scripts/quiet-run web-dayview-fix3 -- bash -lc 'cd web && npm run test -- DayView'` — rc=0。
- `scripts/verify-run 5.3` — Rust の `stay_day_view` と Web の `cd web && npm run test -- DayView` がともに PASS。現行ツリーの証跡を `evidence.jsonl` に追記した。

## Fix report (review 4)

- F1/F2 について、現行 HEAD (`6e8ae58`) の指定検証を再実行した。
- `scripts/quiet-run task5-fix4 -- scripts/verify-run 5.3` — rc=0。
- Rust の `stay_day_view` は PASS、Web の `cd web && npm run test -- DayView` も PASS (rc=0)。現行 commit/tree hash の証跡を `evidence.jsonl` に追記した。
