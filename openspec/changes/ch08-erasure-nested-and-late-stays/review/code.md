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

## code_verify（de2b737..2df3b56。独立検証 2026-10-30）

### 申告: 11/11（tasks.md の `[x]` 1.1〜7.2）・検証コマンド一覧と実測

DB は `ashiato2-ch08-db-1`（healthy）。`.env` を読み込んでから走らせた。`cargo test <絞り込み>` は `-- --list` の本数も数えた。

| 項目 | 検証コマンド（要点） | 申告 | 実測 | 判定 |
|---|---|---|---|---|
| 1.1 | `cargo test purge_hidden_` / `--list ≥ 12` | rc=0 | rc=0・12 passed / 12 本 | 一致 |
| 1.2 | `cargo test purge_` / `stay_` / `stay_ --list ≥ 1` | rc=0 | 7.1 の全体の試験に含まれる（下）・`stay_` は 158 本 | 一致 |
| 2.1 | `cargo test purge_remedy_` / `--list ≥ 8` / `grep -q "deletion::purge_hidden_overlap_remedy(&pool)" lib.rs` | rc=0 | rc=0・10 passed / 10 本 / grep rc=0（**ただし R13**） | 一致（検査は空洞） |
| 3.1 | `purge_restore_` / `--list ≥ 6` / `purge_late_` / `deletion_tests` / openapi の diff | rc=0 | 6 passed・6 本 / 11 passed / 27 passed / diff rc=0 | 一致 |
| 4.1 | `purge_visible_` / `--list ≥ 3` / `purge_api_` / openapi の diff | rc=0 | 3 passed・3 本 / 22 passed / rc=0 | 一致 |
| 5.1 | `purge_nested_` / `--list ≥ 8` / `day_` / openapi / `npx vitest run` | rc=0 | 10 passed・10 本 / `day_` 173 本は全体の試験の中で緑 / rc=0 / 51 files・509 passed | 一致 |
| 6.1 | `npm run test:e2e -- ch08-nested.spec.ts` / 印 ≥ 14 / `st23-purge.spec.ts day-erase.spec.ts` | rc=0 | rc=0・14 passed / 14 / rc=0・16 passed | 一致 |
| 7.1 | `cargo test -p ashiato-server` | rc=0 | **rc=101・1748 passed / 1 failed**（`archive_scan_interval_change_takes_effect_without_restart`。単独では rc=0・1 passed 14.19s） | **不一致**（R14） |
| 7.1 | `check_scenarios.py` / `check-shared-appends.sh` / `openspec validate --strict` | rc=0 | `scenarios: OK` rc=0 / rc=0 / valid rc=0 | 一致 |
| 7.2 | `tools/smoke.sh` | rc=0 | rc=0（103 秒。縦串 OK） | 一致 |

> 縦串は先頭で `docker compose down -v` するので、走らせた後に `tools/db.sh up -d --wait db` と `tools/db-roles.sh` で DB を立て直した（作業ツリーは触っていない）。

### 手 1: 固定値を独立に再計算する —— 該当なし

ハッシュや計算結果を直書きした試験は無い。画面の文面の期待値（e2e の `後から 2 件も本文ごと消した`・`一覧に出ている重なる滞在 1 件は残ります。後で作り直しで隠れたら、その本文も消えます`・
`本文ごと消した時間の中なので戻せません`・`戻しても一覧に戻らないため、戻せません`）は、spec の THEN（`specs/record-deletion/spec.md:190・469・489`）の字面と grep で照らして一致した。実装（`web/src/messages.ts:301-305`）から写した値ではない。

### 手 2: ガードをわざと壊す

1 か所ずつ `sed` で壊し → 絞り込みの試験 → `git checkout -- <file>` で戻した（毎回 `git status --porcelain` が空に戻ることを確かめた）。

| # | 壊したもの | 走らせた試験 | 結果 |
|---|---|---|---|
| M1 | `deletion.rs:271` の `return Err(RestoreError::NotRestorable)` を消す（Q2） | `purge_restore_` | rc=101・4 failed（nested_is_refused ほか） |
| M2 | `stay_store.rs:713` の `take_hidden_overlaps` を呼ばない（Q8） | `purge_hidden_` | rc=101・7 failed |
| M3b | 手当ての条件 3（`NOT EXISTS deletion_ledger`。`deletion.rs:1348`）を `TRUE` に | `purge_remedy_` | rc=101・1 failed（`purge_remedy_restored_then_hidden_stay_keeps_its_body`） |
| M4 | 手当ての条件 4（`ingest_time <> deleted_at`。`deletion.rs:1347`）を `TRUE` に | `purge_remedy_` | rc=101・1 failed（`…stay_created_hidden_after_the_purge…`） |
| M5 | C1: `stay_store.rs:614` の `<` を `<=` に（端が触れるのを重なりにする） | `purge_hidden_` | rc=101・1 failed（`…edge_touching_stay_keeps_its_body`） |
| M6 | C1: `visible_overlaps` の `e.event_time < $4` を `<=` に（`deletion.rs:664`） | `purge_visible_` | rc=101・1 failed（`…touching_edge_is_not_an_overlap`） |
| M7 | `purge_hidden_overlap` の印 `'user:purged'` を `'user'` に（`deletion.rs:1311`。C7） | `purge_hidden_` | rc=101・2 failed（窓を広げない試験を含む） |
| M8 | 入れ子の後からの件数を親に足さない（`stay_store.rs:1811`） | `purge_nested_` | rc=101・1 failed |
| M11 | 消した順を逆に（`ORDER BY txid DESC`。`stay_store.rs:1722`） | `purge_nested_` | rc=101・1 failed（`…same_time_purged_stays_first_one_is_the_span`） |
| M13 | 窓（操作の識別子の無い行）も親の候補にする（`stay_store.rs:1750`。D10） | `purge_nested_` | rc=101・1 failed（`…day_window_without_a_range_row_is_not_a_parent`） |
| M12 | `lib.rs:4233` の `deletion::purge_hidden_overlap_remedy(&pool).await;` をコメントにする / `if false { … }` で包む | Task 2.1 の grep | **どちらも rc=0（落ちない）** → R13 |

（M3 は `AND TRUE OR NOT EXISTS` と書いて演算子の優先順位で壊れ方が広すぎたので、M3b を正とする。）

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch08-erasure-nested-and-late-stays` → `scenarios: OK` rc=0（warn は CH08 の外の既存の印だけ）。
印の先を 1 件ずつ読んだ。主張の階層と試験の階層が違うものが 2 つ:
- 「起動すると…」「手当てが失敗しても起動は止まらず…」: 主張は**起動**、試験は関数の直接の呼び出し。起動からの呼び出しを守るのは grep だけで、それが空洞（R13）
- 「はみ出しが滞在になる消した滞在は戻せる」: 主張は **1 日の並び**の区間、試験は `deleted_at IS NULL` だけ（R15）

### 手 4: 本人の決定が test で固定されているか

- Q2（戻しても一覧に戻らない B は戻さない）: M1 で落ちる。画面の文面は e2e が字面で固定
- Q3（「後から N 件も本文ごと消した」）: `ch08-nested.spec.ts:223`・`st23-purge.spec.ts:373` が字面で固定
- Q4（L があるときだけ添える）: 両分岐を e2e が固定（`:265` の字面・`:278` の不在）
- Q5（起動のたび）: 処理は M3b・M4 で固定。**「起動のたびに呼ぶ」は grep だけが頼りで、呼び出しを無効にしても通る**（R13）
- C1: M5・M6 で落ちる。C5 の 44 px: `ch08-nested.spec.ts:250-251` が幅と高さを `≥ 44` で見る。C7: M7 で落ちる
- （本人の決定ではないが）design D5 の `lock_timeout` 5 秒はどの試験も値を見ない（`purge_remedy_lock_timeout_reports_the_candidates_left` は SQLSTATE 55P03 だけを見る）。指摘にはしない

### 手 5: tasks の `[x]` と実体

上の表のとおり。挙がったテストの前置きはどれも実在し、`-- --list` の本数は下限以上（12 / 10 / 6 / 3 / 10）。`cargo test --test <名前>` 型の指定は無い。
7.1 の全体の試験だけが rc≠0（R14）。

### 手 6: 隙間

- 本命: **CH08 の前に戻された B が、CH08 の後の最初の作り直しで確認なしに本文を消される**（R12。一時の試験で実測）
- 本物の戻す口で戻したはみ出しの B は、同じまとまりの作り直しで時間が 11:05 – 11:30 に書き換わり、P と重ならなくなる。その後に別の消した時間帯へ触れて隠れても `rebuild:absorbed` で本文は残った（一時の試験で実測。指摘にしない）
- 起動の手当ての失敗（錠・台帳）・対象を引けない・何度も走る は試験が持つ（M3b・M4 と既存の試験）

### 手 7: 該当なし

`python3 scripts/plan_fix.py pending openspec/changes/ch08-erasure-nested-and-late-stays` → `[]` rc=0。`correction-review.md` は書かない。

一時の試験（`zz_verify_*`）は `purge_hidden_tests.rs`・`purge_restore_tests.rs` に足して走らせ、写しから戻した。戻した後の `git status --porcelain` と `git diff` は検証の前（どちらも空）と同じ。

## R12. 本人が消して戻した滞在 B が、後の作り直しで隠れると確認なしに本文ごと消される（手当ての経路は残すのに、作り直しの経路は消す）
- 成果物: `crates/server/src/stay_store.rs`（`take_hidden_overlaps`。:600・:608）/ `crates/server/src/deletion.rs`（`remedy_candidates_sql` の条件 3。:1348）/ `specs/record-deletion/spec.md:507・534` / `deep.md` Q2 の帰結
- 根拠: `purge_hidden_tests.rs` に一時の試験を足して実測。spec の Scenario「本文ごと消した後に戻されて作り直しで隠れた消した滞在は起動しても本文が残る」と同じ前の状態（10:00–11:00 を本文ごと消した後、10:15–10:45 の B を `deletion::erase` で消し、印を外して削除の台帳に `restore` を足す）で、手当てでなく `stay_store::rebuild_day` を呼ぶと
  `VERIFY restored-B after rebuild: mark=(true, Some("user:purged")) kept=false emptied=true`（rc=0）。同じ B を手当てに渡すと本文が残る（`purge_remedy_restored_then_hidden_stay_keeps_its_body`。M3b で条件 3 を外すとこの試験が落ちる）。
  `take_hidden_overlaps` は `Mark::Live` だけを見て、削除の台帳を見ない。deep.md Q2 の帰結は「B の本文を消すのは、本人が確認を見て『本文ごと消す』を押したときだけ」、design D5 の表も「P の後に戻され、作り直しで吸収された B は残す（Q2）」。
  通る道: CH08 の戻す口は、同じまとまりの作り直しで B が隠れると D2 が消した後に `kept` の数が合わず NotRestorable で巻き戻すので漏れない（ただし、その守りを名指しで固定した試験は無い）。実際に通るのは、**CH08 が入る前に ST23 の戻す口で戻され**（作り直しが落ちても 200 でログだけ）**一覧に残った B** が、CH08 の後の最初の作り直しで吸収されるとき。本文の消去は戻せない
- kind: conflict
- 提案: `take_hidden_overlaps` でも、削除の台帳に行のある滞在（本人が消したことのある滞在）を外し、上の前の状態で `rebuild_day` を呼んで本文が残る試験を 1 本足す。外さないなら、D2 にこの道を受け入れる理由を書き、Q2 の帰結との食い違いを本人に返す
- 処置: fixed D11 仮（`take_hidden_overlaps` が削除の台帳に行のある L を外す。D5 の 3 と同じ条件。試験 `purge_hidden_restored_stay_keeps_its_body` は直しを外すと落ちる。spec に Scenario「本人が消して戻したことのある重なる滞在は作り直しで隠れても本文が残る」。`provisional.jsonl` に D11）

## R13. 「起動のたびに手当てを呼ぶ」を守るのは grep だけで、呼び出しを無効にしても検証が通る
- 成果物: `tasks.md` 2.1 の 3 本目の検証 / `crates/server/src/lib.rs:4233` / `crates/server/src/purge_remedy_tests.rs`
- 根拠: `lib.rs:4233` を `// deletion::purge_hidden_overlap_remedy(&pool).await;` にしても、`if false { deletion::purge_hidden_overlap_remedy(&pool).await; }` にしても、`grep -q "deletion::purge_hidden_overlap_remedy(&pool)" crates/server/src/lib.rs` は **rc=0**（M12）。
  「起動する」系の Scenario 8 本の試験はどれも `purge_hidden_overlap_remedy_for_user` を直接呼び（`purge_remedy_startup_path_empties_every_users_overlapping_stay` も `deletion::purge_hidden_overlap_remedy(&app)` を直接呼ぶ。`purge_remedy_tests.rs:149・161`）、`run()` を通るものは 0 本。主張の階層（起動）と試験の階層（関数）が違う
- kind: technical
- 提案: 起動の並びを試験から呼べる関数に切り出してそれを試験するか、少なくとも検査を「コメントでない行の、文として呼ぶ形」（例: `grep -qE '^\s+deletion::purge_hidden_overlap_remedy\(&pool\)\.await;'` に加え `if false` 等を拒む）に締める
- 処置: fixed 2.1 （起動時の手入れを `startup_upkeep(pool)` に切り出し、`run()` は待ち受けの前にそれを 1 回呼ぶ。`purge_remedy_startup_path_empties_every_users_overlapping_stay` は `startup_upkeep` を通す。その中の手当ての呼び出しをコメントにするとこの試験が落ちることを確かめた。tasks の grep の形は変わらず通る）

## R14. Task 7.1 の `cargo test -p ashiato-server` が rc=0 にならないまま `[x]`
- 成果物: `tasks.md` 7.1 / `crates/server/src/archive_scan_interval_tests.rs:164`
- 根拠: `scripts/quiet-run ct-full -- cargo test -p ashiato-server` → `test result: FAILED. 1748 passed; 1 failed`（rc=101）。落ちたのは `archive_scan_interval_change_takes_effect_without_restart`（「走査が 7 回に届かない: 6」）。単独では rc=0・1 passed（14.19s）。CH08 はこのファイルにも取り込み器にも触れていない（`git diff de2b737..HEAD --stat`）。final review の前書きも同じ 1 本を記録している
- kind: technical
- 提案: CH08 の外の時刻依存として別の change に出し、7.1 の検証の記録に「この 1 本を除いて緑・単独で緑」と事実のまま書く（`[x]` の根拠を rc=0 と書かない）
- 処置: followup ST13 （CH08 の外の時刻依存の試験。`docs/handoff/ST13.md` に書いた。7.1 の検証の記録は PR 本文に事実のまま書く）

## R15. 「はみ出しが滞在になる消した滞在は戻せる」の試験が 1 日の並びを見ておらず、実際の区間は 10:40 – 11:30 でなく 11:05 – 11:30
- 成果物: `crates/server/src/purge_restore_tests.rs`（`purge_restore_overhang_that_is_a_stay_can_be_restored`。:175）/ `specs/record-deletion/spec.md`（同名の Scenario）
- 根拠: THEN は「その日の 1 日の並びに 10:40 から 11:30 の滞在の識別子を持つ『滞在』の区間がある」。試験は `SELECT deleted_at IS NULL` だけを見る。同じ場面で `stay_store::day_view` を読む一時の試験で実測すると
  `Erased 10:00..11:00` / `Move 11:00..11:05` / `Stay 11:05..11:30 id=B` / `NoRecord 11:30..`（rc=0）。B の識別子を持つ区間はあるが、時間は戻す前（10:40 – 11:30）でなく 11:05 – 11:30。
  ST22 の Scenario「消した滞在を戻すと一覧に戻る」は「消す前と同じ識別子と**時刻**で出る」で、この場面では成り立たない（同じまとまりの作り直しが B を書き換える）
- kind: technical
- 提案: 試験で `day_view` を読み、B の識別子を持つ `Stay` の区間があることと、その時間（11:05 – 11:30）を assert する。spec の THEN の「10:40 から 11:30 の滞在」が時間でなく B を指すことを明記するか、ST22 の Scenario の「時刻」が本文ごと消した時間帯と重なる戻しには当たらないことを書き添える
- 処置: fixed 3.1 （試験は戻した後の `day_view` を読み、B の識別子を持つ「滞在」の区間が 11:05 – 11:30 であることを見る。spec の THEN をその時間に直し、ST22 の「消す前と同じ時刻で出る」を本文ごと消した時間帯と重ならない戻しに絞った）

## security review（de2b737..2df3b56）

見た経路:
- `POST /stays/restore` → `authorize`（`lib.rs:2229`。関数の 1 行目）→ `deletion::restore` の新しい重なりの検査と同じまとまりの作り直し（`deletion.rs:227-275`）→ 409 `not_restorable` / 503 `restore_rebuild_failed`。断りのときは `return Err` で tx が commit されず、作り直しの中の D2 の本文の消去も巻き戻る（3）
- `GET /stays/purge/preview` → `authorize`（`lib.rs:2400`）→ `purge_scope` の `visible_overlaps`（`deletion.rs:658-677`。`scope.user_id` で絞った件数だけ、値はバインド）（3）
- `GET /day` → 既存の認証 → `DayRow.nested`（`stay_store.rs:1520`。出すのは始まり・終わり・滞在の識別子・`purged` だけで、位置・本文・題名は無い）→ `DayView.tsx:458` の `NestedRow`（JSX のテキストで、HTML として差し込まない）（3）
- 起動 → `run()` → `deletion::purge_hidden_overlap_remedy(&pool)`（`lib.rs:4233`）→ 利用者ごとの錠 → `purge_hidden_overlap`。外からの入力は無く、ログ（`warn_remedy`）は `kind`・件数・SQLSTATE だけ（3）
- 戻す口の 503 のログ → `stay_store::failure_kind`（`stay_store.rs:477`。分類名か SQLSTATE だけで、値を含まない）（2）
- 試験の差し込み口 `RESTORE_REBUILD_SCOPE`・`purge_hidden_overlap_remedy_for_user` は `#[cfg(test)]` だけで、本番の組み立てには残らない（`deletion.rs:106-119`・`:1418`）（2）
- router の `.route` は差分で増えていない（`git diff … -- crates/server/src/lib.rs | grep '\.route'` が空。`/day` `/stays/restore` `/stays/purge/preview` は `lib.rs:4086-4097` の既存の行）。新しい listen・送信先・依存（`Cargo.toml` / `package.json` / gradle）も無い（`git diff --name-only` に無い）（2）

安全の決め手: この差分は新しい入口を足しておらず、新しい断り（409/503）と `visible_overlaps`・`nested` はどれも、関数の頭で `authorize` を通った既存の 3 つの handler の後ろにしか無い —— 確かさ 4。`scripts/quiet-run sec -- cargo test -p ashiato-server -- restore_endpoint purge_api_preview_requires_credentials st25_api_day_without_credentials_is_401` → `3 passed; 0 failed`（rc=0）。消した滞在を置いた状態で token なしで `stays_restore` を呼ぶと 401 が返り、記録の印は変わらない（`deletion_tests.rs:341-352`）。重なりの検査より前で断ることは、`authorize` が 1 行目にあることを辿って確かめた（3）。重なりのある戻しを token なしで送って 401 になる試験は、本数としては無い

指摘なし

範囲外: 削除・本文ごと消す口の認証の試験は、router を通さずに handler を直接呼んでいる（`deletion_tests.rs:341`）。router に middleware を付けても付けなくても結果は同じ（ST22/ST23 の既存の形）

## /code-review（de2b737..2df3b56）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R16. The restore check counts a stay as overlapping a body-purged window only on strict overlap (`start_at < $3 AND end_at > $2`), but the rebuild hides a fresh stay that merely touches an erased span (`touches_erased` uses closed intervals). So a stay that only touches a purged window is reported as restored, then hidden again by the rebuild that runs after commit.
- 成果物: crates/server/src/deletion.rs:235
- 根拠: Purged window P is 10:00-11:00 and erased stay S is 11:00-11:30. Restore S: `overlaps` is false, so the in-transaction check is skipped, the transaction commits and the caller returns 200. The post-commit rebuild then sees fresh S touching P (`f.start <= e`) and hides it again as erased-range. The UI clears the row and reloads, and S reappears as "消した". The user gets a success with no effect instead of `not_restorable`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 一時の試験で再現しなかった。P 10:00–11:00 を本文ごと消し、11:00–11:30 の S（11:05〜11:30 の位置）を戻す → 200、その後に本物の `rebuild_day` → S は `deleted_by` NULL のまま、1 日の並びは `Stay 11:05–11:30`。作り直しは P の両端の位置に後着の印を付けてから滞在を作る（design D2 の (b) の節）ので、新しい滞在は P に触れず、`touches_erased` で隠れない。spec の Scenario「本文ごと消した時間と境界で位置を共有しても隣の消した滞在は戻せる」がこの場面。理由は design D4 に書き足した

## R17. The startup remedy query drops any stay that has ever had a deletion-ledger row (`NOT EXISTS (SELECT 1 FROM core.deletion_ledger d WHERE d.event_id = e.id)`). A stay that was erased and later restored, and then hidden by a pre-CH08 rebuild under a purged window, is never remedied.
- 成果物: crates/server/src/deletion.rs:1364
- 根拠: Stay X is erased and restored, so the ledger holds erase and restore rows. The user then body-purges overlapping stay Y before CH08 is deployed, and a rebuild absorbs X (`rebuild:absorbed`, raw kept). After deployment, every startup excludes X because ledger rows exist, so X's body stays forever inside a time range the user body-purged.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 意図した見逃し（`design.md:114`・`design.md:134`・`specs/record-deletion/spec.md:541`）。spec review R2〜R4 で、本人が消したか戻したことのある滞在は時刻を比べずに外すと決めた（design D5 の 3 と表の最後の行。「見つけ損ねはどれも残す側」）。戻したのが P の前か後かは時刻を比べないと分からず、P の後に戻された B を確認なしに消すと Q2 に反する。spec の Scenario「本文ごと消した後に戻されて作り直しで隠れた消した滞在は起動しても本文が残る」。反転条件は D5 にある

## R18. In `take_hidden_overlaps`, the parent window is picked from purged rows only, but the 'hiding' test uses `touches_erased(f, erased)`, and `erased` includes ordinary (non-purged) user-erased stays. A Live stay can lose its body even when its successor is hidden only by a restorable erased stay, not by the purged window.
- 成果物: crates/server/src/stay_store.rs:628
- 根拠: Live stay L partly overlaps purged window P. The fresh stay f that inherits L has shifted off P but touches an ordinary erased stay E. `hiding` matches, `inherits` is cleared and L's body is purged under parent P, although f is hidden only by E (which the user could restore). Data is irreversibly purged that D2 says should only be hidden.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D2 の (b) がこの場面そのもの（「`f` が隠れるのは、P と別の、本文を消していない消した時間帯に触れるとき」）で、spec の Scenario「作り直しで消した時間帯と重なるとして隠れた重なる滞在の本文も消える」が試験で固定している。消えるのは P と重なる時間を持った L の本文だけで、`f` は生きた位置から新しい行として隠して作るので、E を戻せば `f` は戻る

## R19. A non-purged erased stay fully covered by a whole-day window (an operation-less D14 row) is put into `covered_erased`. `parent_of(&parents, ...)` then returns None and the code does `continue`, so the stay appears neither as a nested row nor as a fragment and silently disappears from the day view.
- 成果物: crates/server/src/stay_store.rs:1884
- 根拠: A day holds a purged stay with no span row (a whole-day window) plus an ordinary erased stay at 14:00-15:00. `subtract_spans` against `purged_spans` (which includes the window) is empty, `parents` has no entry overlapping it, and the 14:00-15:00 erased row, with its restore or purge controls, is gone from the list. This contradicts CH08's goal of keeping enclosed stays visible.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D12 仮（`day_view` は親の見つからない、窓だけに覆われる B を差し引かずに自分の「消した」の区間として並べる。一時の試験で再現 → 試験 `purge_nested_day_window_does_not_drop_an_erased_stay` は直しを外すと落ちる。`provisional.jsonl` に D12）

## R20. The remedy picks parent windows from `core.erasure_operation.start_at/end_at`, while the inline rebuild path picks them from `core.stay_erased_purge` rows (the event-scope span of the first operation, and only rows still in the view). The two code paths can attach the same hidden stay to different parent operations, and so to different `late_purged` counts.
- 成果物: crates/server/src/deletion.rs:1475
- 根拠: Two user operations overlap a hidden stay with equal overlap, and one operation's purged stay sits outside the rebuild's [lo, hi] load range (or its span row differs from the op range). The rebuild picks op A as `parent_id`, while the startup remedy, which sees all user ops, picks op B. The day view then adds the `+1 後から` count to a different parent row depending on which path ran.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 2 つの経路の窓は同じ値になる。本人の操作の `start_at`・`end_at` と、指定した滞在の範囲の行（`scope = 'event'`）は、本文ごと消す口が同じ `scope.start`・`scope.end` から書く（`deletion.rs` の `purge_using_span_scope`）。本文を消した滞在は戻せず `core.stay_erased_purge` から抜けない。作り直しの `load_existing` は L の時間を含む範囲を読むので、L と重なる P はどれも読む。同順の決め手（重なり → 始まり → 操作の識別子）も同じ（R24 で 1 つの関数にした）

## R21. The startup remedy runs synchronously before the server binds. It runs a full candidate scan and count, then one transaction per user with a 5-second `lock_timeout`, so startup latency grows with data volume and lock contention.
- 成果物: crates/server/src/lib.rs:4233
- 根拠: With many users, or a held advisory lock (a concurrent rebuild worker during a rolling restart), each user can wait up to 5 seconds and the scan reads every absorbed stay. The HTTP listener is delayed by N×5 seconds and health checks fail. Running it in a spawned background task after listen would avoid blocking startup.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D5 が待ち受けの前で呼ぶと決め、Risks に「錠を待つと待ち受けの開始が遅れる → `lock_timeout` 5 秒で諦めて次の起動に回す」と書いた。取り込み器と定期取得はこの後に起こすので、同じプロセスの作り直しと錠を取り合わない。候補は final review R3 の粗い絞りで、本人の操作と重なりうる行だけを読む。利用者は本人 1 人（自分のための記録）

## R22. The nesting loop is O(n^3): on every iteration of the `while let`, for each candidate i, it rebuilds and sorts a fresh `others` Vec of every other kept span.
- 成果物: crates/server/src/stay_store.rs:1758
- 根拠: A day with dozens of purged rows does about n^2 allocations and sorts per removal and n removals on every day-view request. A cheaper option: sort once, and test coverage against a prefix-merged union that excludes i, or sweep in rank order.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `crates/server/src/stay_store.rs:1775` の n はその日の、操作の識別子を持つ本文ごと消した滞在の数。1 日に数件で、数十件でも 1 回の読み出しで数万回の区間の比べにとどまる。並びの規則（消した順の遅い方から 1 つずつ外す）を崩さずに速くする手間に見合わない

## R23. The restore runs one `EXISTS` query against `core.stay_erased_purge` per restored stay inside the loop, instead of a single set-based query over all restored spans.
- 成果物: crates/server/src/deletion.rs:237
- 根拠: Restoring a merged erased row with many `stay_ids` issues N round-trips to the view (each with LATERAL joins) while holding the user's advisory lock. One `unnest($starts, $ends)` EXISTS/ANY query would return the overlapping ids in a single round-trip.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 （戻した滞在の重なりを `unnest` の 1 回の問い合わせで引く。`purge_restore_` は緑）

## R24. The rule for choosing a parent window (largest overlap, then earliest start, then id) is written three times: `take_hidden_overlaps`, the closure in `remedy_user`, and `parent_of` in stay_store. The same SQL end-time expression from the `core.stay_erased_purge` view is also copied into `purge_scope` and `remedy_candidates_sql`.
- 成果物: crates/server/src/deletion.rs:1480
- 根拠: A future change to D3/D6 tie-breaking, or to how the end time is read (for example ST41's end_time precedence), has to be made in 3 Rust places and 3 SQL places. Missing one makes the rebuild, the remedy and the day view disagree on the parent. Factor out a single `pick_parent(start, end, windows)` helper and a SQL function such as `core.stay_end(e)`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.1 （親の選び方を `stay_store::pick_window` の 1 つにし、`take_hidden_overlaps`・`remedy_user`・`parent_of` が使う）。SQL の終わりの読み方は rejected: 手当ての絞りは意図して違う形（`GREATEST` と上限なし。D5 の上位集合）で写しではない。`purge_scope` とビューの写しを 1 つの SQL 関数にするには移行が要り、この change の範囲（Q1）を越える

## R25. Test-only fault injection is threaded through production signatures: the `ledger_scope: &str` parameter on `rebuild_day_in`, `purge_hidden_overlap` and the remedy, plus a `thread_local!` `RESTORE_REBUILD_SCOPE`. Production callers have to pass the magic string "event".
- 成果物: crates/server/src/stay_store.rs:694
- 根拠: Every production caller carries a meaningless `"event"` argument, and a wrong literal compiles silently into a CHECK violation at runtime. A single `#[cfg(test)]` hook, like the existing thread_local, used consistently would keep the production API clean.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.1 （`ledger_scope` の引数を `rebuild_day_in`・`purge_hidden_overlap`・手当てから外し、試験の差し込みは `#[cfg(test)]` の `HIDDEN_LEDGER_SCOPE` 1 つにした。戻す口の 503 の試験は `FAIL_REBUILD_IN` で落とす（D11 で戻した B の本文を消さなくなり、台帳の差し込みでは落ちなくなったため）。本番の呼び出しは `"event"` を渡さない）
