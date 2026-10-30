# code review — ch08-erasure-nested-and-late-stays

## final review（de2b737..d89440e）

final reviewer の返答の逐語は `.superpowers/sdd/ch08-final-findings.md`（Critical 0 / Important 2 / Minor 7。判定は With fixes）。
fix は 1 回（`449c7170` コード・`53e03c22` 文書）。scoped re-review（d89440e..53e03c2）は I1・I2・M1・M2・M4〜M7 を ADDRESSED、
新しい Critical / Important は無し。re-review が新しく挙げた Minor 2 件は R10・R11 に写した。

全体の試験（`cargo test -p ashiato-server`）は 1748 passed / 1 failed。落ちたのは `archive_scan_interval_change_takes_effect_without_restart`
（実時間で走査の回数を待つ試験）だけで、単独では 5 本とも通る。CH08 は取り込み器に触れておらず、
ST50・verify の木でも負荷の下で同じ試験が落ちている（`.superpowers/sdd/logs/*-ct.log`）。CH08 の外の既存の時刻依存として扱う。

## R1. 書き直した ST23 の試験の本数が tasks・CH08.md（3 本）と design D1（4 本）で食い違い、4 本目の帰属も違う
- 成果物: `openspec/changes/ch08-erasure-nested-and-late-stays/tasks.md`・`docs/changes/CH08.md`・`design.md` D1・`crates/server/src/purge_tests.rs`
- 根拠: tasks.md:17・:130 と CH08.md が「3 本」。D1 の 4 行目と purge_tests.rs:1975 のコメントが `purge_range_reader_allowlist` を戻す口（3.1）の読みとしていたが、実際に許可の一覧へ足したのは D7（4.1）の `visible_overlaps` の数え
- kind: technical
- 処置: fixed D1
- 補足: tasks.md・CH08.md を 4 本に、D1 の帰属を D7 / 4.1 に、コメントも直した。`53e03c22`・`449c7170`

## R2. 範囲の行が無い本文ごと消した窓（ST23 D14。その日全体）が D6 の入れ子でその日全体の親になる
- 成果物: `crates/server/src/stay_store.rs`（`day_view` の `op_rank` と `kept`）
- 根拠: `operation_id = None` の行が `Option<usize>` の並びで先頭に来て、その日のほかの本文ごと消した区間・B・後からの件数を全部取り込む。D2 は窓を使わないと決めているが D6 は触れていない
- kind: conflict
- 処置: fixed D10 仮（窓は親の候補と覆う側から外し、ST23 のとおり自分の区間として並べる。試験 `purge_nested_day_window_without_a_range_row_is_not_a_parent`。`provisional.jsonl` に D10）

## R3. 起動時の手当ての候補が、手当てと関係のない吸収済みの滞在まで毎回すべて引く
- 成果物: `crates/server/src/deletion.rs`（手当ての候補の SQL）
- 根拠: D5 の条件 1・3・4 は CH08 の後のふつうの吸収済みの滞在にもほぼ当てはまり、錠の前の件数も過大になる
- kind: technical
- 処置: fixed D5
- 補足: `kind='user'` の操作と重なりうる行だけに SQL で絞る `EXISTS` を足した。終わりは `GREATEST(event_time+1µs, end_time, coalesce(payload の end, infinity))` で `span_of` の比べの上位集合。re-review が D5 の対象を縮めないことを確かめた

## R4. 本番の経路 `purge_hidden_overlap_remedy(&pool)`（利用者で絞らない）を呼ぶ試験が無い
- 成果物: `crates/server/src/purge_remedy_tests.rs`
- 根拠: grep で呼び出しが `run()` の 1 か所だけ
- kind: technical
- 処置: fixed 2.1
- 補足: `purge_remedy_startup_path_empties_every_users_overlapping_stay`。使い捨ての DB（`testdb::isolated_with_app`）でアプリの役割から呼ぶ

## R5. 吸収された L を本文ごと消しても `core.stay_absorbed` に元の行が残り、連鎖を読む側が辿りうる
- 成果物: `crates/server/src/deletion.rs`（`purge_hidden_overlap`）
- 根拠: `stay_absorbed` を読むのは `feelings.rs:665-684` だけ
- kind: technical
- 処置: rejected: `feelings.rs:665-684` は印が `Absorbed` の行だけを `stay_absorbed` で辿る。`user:purged` になった L はそこで連鎖が止まり、`resolve_stay` が印で扱う。`stay_absorbed` は追記のみ（`stay_absorbed_append_only`）で、行を消す道も無い。`export.rs:625` は内部の表として書き出さない

## R6. 試験の主張が弱い（空でも通る `.all`、ログの試験が時刻を見ない）
- 成果物: `crates/server/src/purge_hidden_tests.rs`・`purge_remedy_tests.rs`
- 根拠: `purge_hidden_tests.rs:143` の `.all(...)`、`purge_remedy_tests.rs:240`
- kind: technical
- 処置: fixed 1.1
- 補足: `assert_eq!(erased, vec![(10:00, 11:00)])` に、ログの試験は日本時間と UTC の時刻の文字列が出ないことを見る形に

## R7. コメントが実際の失敗点と違い、到達しない `else continue` が不変条件の崩れを黙って通す
- 成果物: `crates/server/src/stay_store.rs`
- 根拠: `stay_store.rs:667` のコメント、`:752` の `let Some(parent_id) … else continue`
- kind: technical
- 処置: fixed 1.1
- 補足: コメントを手順 2 に直し、`ok_or_else(...)?` で作り直しごと巻き戻す

## R8. `visible as u32` の黙った切り詰め・画面の別名の定数・e2e の使わない欄
- 成果物: `crates/server/src/deletion.rs`・`web/src/DayView.tsx`・`web/e2e/ch08-nested.spec.ts`
- 根拠: `deletion.rs:675`、`DayView.tsx:378` の `const base = ROW_BASE`、e2e の `Entry`
- kind: technical
- 処置: fixed 4.1
- 補足: `u32::try_from` で断る。定数は直接使い、使わない欄と連続する空行を片付けた。6.1 の分も同じ commit

## R9. 試験の置き場が design D9（2 ファイル）と実装（5 ファイル）で違う
- 成果物: `design.md` D9・`tasks.md` の前文
- 根拠: 実装は `purge_hidden_tests`・`purge_remedy_tests`・`purge_restore_tests`・`purge_visible_tests`・`purge_nested_day_tests`
- kind: technical
- 処置: fixed D9
- 補足: D9 の表と tasks の前文を 5 本に直し、変えた理由を書いた

## R10. B が A と一部だけ重なり、残りが窓にだけ覆われると、はみ出しているのに A の入れ子になる（re-review の Minor）
- 成果物: `crates/server/src/stay_store.rs`（`day_view`）
- 根拠: re-review。A 10:00–11:00・B 10:30–11:30・同じ日に窓のとき、`subtract_spans` で B が空になり `parent_of(&parents)` が A を返す
- kind: technical
- 処置: rejected: `stay_store.rs:1750` が窓（`operation_id` 無し）を `kept` から分け、`:1775` の `parents` に入れない。B のはみ出し（11:00–11:30）は窓（その日全体の本文ごと消した時間）の中にあり、B 全体が本文ごと消した時間に覆われている。D6 の 2「残りの区間の和に覆われる B は重なりの大きい方の下」と同じ形で、窓は親にならない（D10）ので A が親になるのは D6・D10 どおり

## R11. `payload->>'end'` にマイクロ秒より細かい端数があると、手当ての粗い絞りから漏れうる（re-review の Minor）
- 成果物: `crates/server/src/deletion.rs`（手当ての候補の SQL）
- 根拠: re-review。PG は µs に丸め、chrono はナノ秒のまま読む。`op.start_at` との差が 1µs 未満のときだけ起きる
- kind: technical
- 処置: rejected: 対象は `origin='derived'` の滞在で、`payload` の end はサーバが DB の時刻（µs 精度）から書く。µs より細かい端数を持つ行は作られない
