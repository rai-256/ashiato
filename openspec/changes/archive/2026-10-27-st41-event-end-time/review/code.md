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
- 処置: fixed 7.2 —— `docs/briefs/ST41-pr.md` の「計画外の変更」と「検証」の節に、`202609120943` の編集・`check-openapi.sh` の除外・`.last()` → `.any()`・`check-immutable.sh` の戻し手順の節への挿入と、7.2 の証跡の出どころを書いた

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

## code_verify（HEAD 9935e831。2026-10-25 実測）

席: code_verify（手 1〜7）。作業ツリーは clean（`git status --porcelain` が空）の HEAD `9935e831`（origin/main `6bb4584e` の上に rebase 済み）で走らせた。
壊したコードと移行はすべて `git checkout --` で戻し、戻した後の `git status --porcelain` は空・`git diff | sha1sum` は壊す前と同じ `da39a3ee…`（違いはこのファイルだけ）。
共有の DB の関数を壊した分は、元の移行を当て直して戻した（`pg_proc` で確認）。check-immutable と smoke が DB を消したので、最後に `tools/db.sh up` → `tools/db-roles.sh` で立て直した（rc=0）。

### 申告と実測

申告: **18/18**（`- [x]` 18 件）。

| タスク | 検証コマンド（tasks.md） | 実測 | 判定 |
|---|---|---|---|
| 1.1 | `./tools/check-migrations.sh`・`cargo build -p ashiato-server` | rc=0「マイグレーション OK」/ 全体の試験の中でビルドが通った | 一致 |
| 1.2 | `CT st41_gate_ (≥6)` | `--list` 9 本・全体の走りで 9 ok | 一致 |
| 1.3 / 7.2 | `./tools/check-immutable.sh`・`./tools/check-shared-appends.sh` | rc=0「書き換え禁止 OK」/ rc=0「共有の本体 OK」 | 一致（ただし R15） |
| 2.2 | `CT st41_ingest_ (≥13)`・`CT dedup_ (≥30)` | 15 本 / 39 本・全部 ok | 一致 |
| 2.3 | `check-openapi.sh`・`jq -e …has("end_time")…`・`git diff --quiet -- collector-android crates/collector-windows` | rc=0「API の契約 OK」/ `true` rc=0 / rc=0 | 一致 |
| 3.1 | `cargo test -p ashiato-server --lib st41_hash (≥1)` | 1 本 ok | 一致 |
| 3.3 | `CT st41_update_ (≥8)`・`CT st41_dedup_ (≥3)`・`CT dedup_ (≥30)` | 8 / 3 / 39 本・全部 ok | 一致 |
| 4.1 | `CT st41_coverage_ (≥3)` | 3 本 ok | 一致 |
| 5.2 | `CT st41_range_ (≥16)`・`CT st25_api_unchanged (≥1)` | 18 / 1 本 ok | 一致 |
| 5.2 | `git diff -U0 origin/main -- crates/server/src/day_records_tests.rs \| grep … ` の出力が空（rc=1） | **出力あり rc=0**（呼び方の折り返し 5 行と `.filter(\| r \| ["c02-window", "c01-location"]…)` の 7 行） | **不一致**（R12 の裁定のまま。新しい R にしない） |
| 5.3 | `CT st41_range_uses_partial_index (≥1)`・`check-openapi.sh` | 1 本 ok / rc=0 | 一致 |
| 6.2 | `CT st41_export_ (≥7)`・`CT export_ (≥130)` | 8 / 166 本・全部 ok | 一致 |
| 7.1 | `git log … --grep 'ch02' … \| grep -q . \|\| test -f docs/handoff/CH02.md` | rc=0（`ba2a9ed0 feat(ch02): …` が main にある） | 一致 |
| 7.1 | `python3 scripts/review_triage.py . st41-event-end-time` | **rc=1**「code.md R13: 処置が … のどれでもない」 | **不一致**（R13 の `処置:` が finish 待ちで空。finish で埋まる。この席は処置を書かない） |
| 7.1b | `git diff --quiet origin/main -- stay_store.rs collector-android crates/collector-windows`・`CT st41_stays_and_collected (≥1)` | rc=0 / 1 本 ok | 一致（ただし R14） |
| 7.2 | `cargo fmt --all -- --check`・`cargo clippy --workspace --all-targets -- -D warnings` | rc=0 / rc=0 | 一致 |
| 7.2 | `cargo test -p ashiato-server` | rc=0（lib 1452 passed / server_startup 9 passed、2032 s。`.superpowers/sdd/logs/20261025-081505-cvfull.log`） | 一致 |
| 7.2 | `check-migrations && check-boundaries && check-openapi && check-immutable` | 4 本とも rc=0 | 一致 |
| 7.2 | `scripts/quiet-run smoke -- ./tools/smoke.sh` | 1 回目 rc=1「受理の並びが違う」（echo は `[true,false,true]`）。同じ時刻に ST23 の worktree の `tools/smoke.sh` が走っていて `/tmp/smoke.body` を取り合った（`ps` で確認・`/tmp/smoke.body` の mtime が相手の走りの中）。相手が終わってから 2 回目 rc=0「縦串 OK」 | 一致（注 1） |
| 7.2 | `python3 scripts/check_scenarios.py . st41-event-end-time` | rc=0「scenarios: OK」（warn は ST41 の外の印だけ） | 一致 |

- 注 1: `/tmp/smoke.body` は worktree をまたいで共有される固定の名前（`tools/smoke.sh` は ST41 の持ち物ではない）。並走する Story の smoke が互いの応答を読む。ST41 の指摘にはしない
- 注 2: 2.1 / 3.2 / 5.1 / 6.1 の「いまの木で落ちることを確かめてから」は実装前の木の話で、今の木では確かめられない（手 4 の壊し方で、同じ試験が実装を外すと落ちることは確かめた）
- 注 3: 全体の走りの後、素の木で `day_tests::st25_reasons_before_collection_started` が単独で落ちる（`c01-location` の `collection_started_on` が別の試験の残した `2000-01-07`）。ST13 の code_verify の注 1 と同じ共有 DB の積もりで、ST41 の差分（日付は 2026 年）とは無関係

### 手 1: 固定値を独立に再計算する

一致。`content_hash_of` の入力（論理ソース `test`・`1_757_000_000` 秒を µs の i64 BE・原文 `{"v":"x"}`、各項目に u64 BE の長さを前置）を python の `hashlib.sha256` と `struct.pack` で組み直して
`39d0ebc5c3c1d17a5deec93e03e60c3be02e5ddd741a2d5575b789c16ee15df1` —— `st41_hash_with_end_is_pinned` と tasks 3.1 の値に一致。tasks 3.1 の終わり 2025-09-04T16:33:20Z も `1_757_000_000 + 3_600` を python の `datetime` で直して一致。

### 手 2: ガードをわざと壊す

- `check-immutable.sh` を、移行（`202610231200_event_end_time.sql`）の作業ツリーの写しを 1 行ずつ壊して走らせた:
  - 門の `content_changed` から `OR NEW.end_time IS DISTINCT FROM OLD.end_time` を外す → **rc=1**（「NG 終わりの時刻だけを履歴なしで変えられた」ほか）
  - 門の版の照合から `AND v.end_time IS NOT DISTINCT FROM OLD.end_time` を外す → **rc=1**（「NG 終わりだけ違う履歴の 1 行で終わりの書き換えが通った」）
  - 前の版の錠 `reject_version_rewrite` から `OR NEW.end_time IS DISTINCT FROM OLD.end_time` を外す → **rc=0「書き換え禁止 OK」** → **R15**
- 同じ 3 つ目の壊し方を Rust の側で: 移行のファイルを壊した木で `cargo test -p ashiato-server st41_gate_` → rc=101（`st41_gate_version_end_is_frozen` が `end_time_tests.rs:254` の「履歴は追記のみ」の本文の照合で落ちる）。Rust の試験は錠を見ている
- `check-openapi.sh` の除外（終わりの 3 欄を収集アプリの欄名の照合から外す）は名前を 3 つ名指しで外すだけで、ほかの欄は従来どおり見る（読んで確認）。`check-migrations.sh` は前進側だけを見る作りで、`.down.sql` の `DROP COLUMN` は 1 行目に「不可逆」を明記している

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . st41-event-end-time` → rc=0。印の先を、手 4 の壊し方（下の表）で 1 件ずつ当てた。主張の階層が違うのは 2 つ:
7.1b の `st41_stays_and_collected_rows_have_no_end_column` は収集済みの記録を取り込み口を通さず SQL で直に置くので「収集済みの記録を写さない」を観測していない（**R14**）、
check-immutable の「履歴の前の版の終わりは書き換えられない」は錠の手前の別の拒否で OK になる（**R15**）。

### 手 4: 本人の決定が test で固定されているか

壊した木で `cargo test -p ashiato-server <絞り込み>` を走らせ、戻した。

| 決定 | 壊し方（file） | 結果 |
|---|---|---|
| Q7 鍵は変えない・外部の識別子のある記録だけ更新 | `lib.rs` の `external_id.is_some() && is_update(…)` から `external_id.is_some() &&` を外す | `st41_dedup_end_only_difference_folds_without_external_id` が落ちる |
| D18（仮）終わりを持たない同じ版の再送で終わりを消さない | `is_update` から `req.end_time.is_some() &&` を外す | `st41_update_resend_without_end_keeps_end` が落ちる |
| D7 終わりのずれと TZ だけの違いは更新にしない | `is_update` に `end_tz_id` / `end_tz_offset_min` の比較を足す | `st41_update_end_tz_only_change_is_not_an_update` が落ちる |
| D7 µs に切り捨ててから比べる | `ingest_request` の `req.truncate_end_to_micros();` を消す | `st41_update_sub_micro_end_change_is_not_an_update` が落ちる |
| C1 ずれは −1,439〜1,439 | `validate_end` の範囲を ±1,440 に | `st41_ingest_validate_end_boundaries`・`st41_ingest_end_offset_out_of_range_is_rejected` が落ちる |
| Q2 点は `[from, to)` | 点の側の `event_time < $2` を `<=` に | `st41_range_point_is_in_one_of_adjacent_periods` が落ちる |
| Q2 幅は半開区間で重なる | 幅の側を `event_time <= $2 AND end_time >= $1` に | `st41_range_touching_edges_do_not_overlap` が落ちる |
| D8 長さ 0 は点 | 点に倒す条件 `end_time <= event_time` を `<` に | `st41_range_zero_length_is_a_point` が落ちる |
| Q4 件数は始まりの日（Asia/Tokyo） | 稼働記録の日の `$3` を `req.end_time.unwrap_or(req.event_time)` に | `st41_coverage_spanning_event_counts_on_start_day` が落ちる |
| D11 / D4 列の終わりが記録の時刻と同じなら内容から読む | `excluded.rs` の `r.end_time > r.event_time` を `>=` に | `st41_export_end_column_equal_to_start_falls_back_to_payload` が落ちる |
| C3 前の版の終わりは錠の中 | 移行の `reject_version_rewrite` の `end_time` の行を外す | Rust は落ちる / check-immutable は通る（R15） |
| Q5 滞在は列に書かない | （壊さず読んだ）7.1b が `stay_store::rebuild_all` の後に滞在と前の版の終わりの列が 0 件を見る | ある |
| Q1 逆順の終わりは送られたまま入れ、期間では始まりの点 | （壊さず読んだ）`st41_ingest_end_before_start_is_kept`・`st41_range_reversed_end_is_a_point` が値と返る期間を直に見る | ある |
| Q3 `GET /events` に `from` / `to` / `source` | （壊さず読んだ）`st41_range_*` 18 本がハンドラと router を通す | ある |
| **Q6 収集済みの記録の中身の終わりを列に写さない** | `ingest_request` の切り捨ての直後に「`end_time` が無ければ `payload` の `end` / `range_end` を列へ写す」を足す | **`st41_` 66 本 rc=0。全体は 1449 passed / 3 failed で、落ちた 3 本は Q6 を観測していない → R14** |

### 手 5: tasks の `[x]` と実体

18 件とも検証コマンドが実在し、`CT` の絞り込みはどれも下限以上の本数がある（上の表）。`cargo test --test <名前>` の形は tasks に無い。
不一致は 2 つで、どちらも既にある R が持つ: 5.2 の `git diff … | grep` が空にならない（R12 の裁定）、7.1 の `review_triage.py` が rc=1（R13 の処置が finish 待ち）。

### 手 6: 隙間

確かめた範囲: 外部の識別子の更新の経路（古い版・削除済み・水位）・取り込みの 1 件ごとの断り・移行の当て直し（起動のたびに全版を当てる。`202609120943` のビューを `to_regclass` で囲んだ直し）・`.down.sql`（「不可逆」を明記）・書き出しの列。
- 外部の識別子の無い記録の後から来た終わりは畳まれて捨てられ、応答は `duplicate: true` の受理（Q7 の本人の答えで受け入れた損）。`docs/handoff/ST39.md` 17〜22 行に宛先と「失われるもの」がある —— 隙間には落ちていない
- R4（期間の点の側の索引）は `deferred ST45`。`docs/handoff/ST45.md` には無いが、`story_brief.py` の `handoffs_for` が `openspec/changes/**/review/*.md` の `処置: deferred ST45` を ST45 のブリーフへ拾う（archive の下も glob に入る）—— 受け皿はある
- 消した滞在の時間の読み口が、逆順の列の終わりを内容の終わりより先に読む（**R16**）

### 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st41-event-end-time` が `[]`）。

## R14. 本人の決定 Q6「収集済みの記録の中身にある終わりを列に写さない」を直接観測する試験が無い（壊した木の全体で落ちた 2 本は取り込み口を通らない巻き込み）
- 成果物: crates/server/src/end_time_tests.rs（`st41_stays_and_collected_rows_have_no_end_column`）/ crates/server/src/lib.rs（`ingest_request`）
- 根拠: `lib.rs` の `req.truncate_end_to_micros();` の直後に「`end_time` が無ければ `payload` の `end` / `range_end` を RFC 3339 で読み、始まりのずれと TZ と一緒に列へ写す」を足した木で `cargo test -p ashiato-server st41_` → rc=0（66 passed）。全体 `cargo test -p ashiato-server` は 1449 passed / 3 failed だが、落ちた `st41_export_end_column_span_is_excluded`・`st41_export_end_column_wins_over_payload` はどちらも行を SQL で直に置き取り込み口を通らない試験（素の木の単独の走りで 8 本とも ok、壊した木の `st41_` の走りでも ok）、もう 1 本 `st25_reasons_before_collection_started` は素の木でも落ちる（注 3）—— Q6 を観測して落ちた試験は無い。7.1b の試験は `c01-location` の行を `INSERT INTO core.event` で直に置き（中身に `end` を持たない）、取り込み口を通らないので、写す実装が入っても 0 件のまま
- kind: technical
- 提案: 7.1b の試験に、取り込み口（`crate::ingest`）経由で中身に `end` を持つ `c01-app-usage` の記録と `range_end` を持つ `c02-window` の記録を 1 件ずつ送り、格納された行の終わりの 3 列が NULL のままであることを見る
- 処置: fixed D14 —— `end_time_tests.rs` の `st41_collected_payload_end_is_not_copied_to_column`（取り込み口を通して `c01-app-usage` の `end` と `c02-window` の `range_end` を送り、終わりの 3 列が NULL のままを見る。終わった後は論理削除して共有の DB に生きた行を残さない）

## R15. check-immutable の「履歴の前の版の終わりは書き換えられない」は、前の版の錠から `end_time` を外しても OK になる
- 成果物: tools/check-immutable.d/202610231200_st41-event-end-time.sh / migrations/202610231200_event_end_time.sql（`core.reject_version_rewrite`）
- 根拠: 移行の写しの `reject_version_rewrite` から `OR NEW.end_time IS DISTINCT FROM OLD.end_time` の 1 行を外した木で `./tools/check-immutable.sh` → **rc=0**「OK 前の版の終わりの書き換えは拒まれた: end_time='2026-09-08T05:00:00Z'」「書き換え禁止 OK」。節の UPDATE は `raw` / `payload` を残したまま終わりだけを変えるので、名指しの列の照合を抜けても、同じ関数の後ろの「履歴の書き換えは本文の消去だけが通る」で拒まれる。錠の名指しが効くのは本文の消去と一緒に終わりを動かす書き込みだけで、節はそれを試していない。同じ壊し方を Rust の `st41_gate_version_end_is_frozen` は本文「履歴は追記のみ」の照合で落とす（rc=101）。tasks 1.3 は「アプリを通さない経路」の守りとしてこの節を置いた
- kind: technical
- 提案: 節の 3 つの UPDATE を、消去の台帳の行を同じまとまりに置いたうえで `raw = '', payload = '{}'` と一緒に終わりの列を変える形にする（拒まれる理由が名指しの列であることを本文の「履歴は追記のみ」で確かめる）
- 処置: fixed 1.3 —— `tools/check-immutable.d/202610231200_st41-event-end-time.sh` の前の版の 3 つの UPDATE は、拒否の本文に「履歴は追記のみ」（錠の名指し）を持つことまで見る。錠から `end_time` の名指しを外した木で `./tools/check-immutable.sh` → rc=1「NG 前の版の終わりを拒んだのが錠の名指しでない」を確かめて戻した

## R16. 消した滞在の時間の読み口が、始まりより前の列の終わりを内容の終わりより先に読み、幅が空になる
- 成果物: migrations/202610231200_event_end_time.sql（`core.stay_erased` の `end_at`）/ crates/server/src/export/excluded.rs
- 根拠: テスト用 DB で、トランザクションの中に `s01-stay`・`derived`・`deleted_at = now()`・`event_time = 01:00Z`・`end_time = 00:30Z`・中身 `{"end":"02:00Z"}` の行を置いて `SELECT start_at, end_at, end_at > start_at FROM core.stay_erased` → `01:00 | 00:30 | f`（ROLLBACK で戻した）。`coalesce(end_time, try_timestamptz(payload->>'end'), …)` は逆順の列の値をそのまま使い、中身の 02:00 を読まない。記録の側の `overlap_sql` は `CASE WHEN r.end_time > r.event_time THEN r.end_time END` で逆順の列を読めないものとして内容へ進む（D4・D11、spec「列の終わりが記録の時刻より前なら、内容の終わりから読む」）ので、対になっていない。いまは `stay_store` が滞在に列を書かない（Q5・7.1b）ので実害は出ていない（`/ingest` が `derived` の `s01-stay` を断るかは確かめていない）
- kind: technical
- 提案: `stay_erased` の `end_at` と `excluded.rs` の消した滞在の前の版の `coalesce(v.end_time, …)` を、`CASE WHEN end_time > event_time THEN end_time END` を先頭にした形にそろえる（D4 の読み方）。試験は `st41_export_erased_stay_end_column_is_read` の対として、逆順の列と正しい中身の終わりを持つ消した滞在で記録が除かれることを見る
- 処置: fixed D11 —— `core.stay_erased` の `end_at` と `excluded.rs` の消した滞在の前の版を `coalesce(CASE WHEN end_time > event_time THEN end_time END, 内容の end, …)` にそろえた（D4 の読み方）。試験 `st41_export_erased_stay_reversed_end_column_falls_back_to_payload`・`st41_export_erased_stay_version_reversed_end_column_falls_back_to_payload`

## security review（6bb4584..9935e83）

見た経路:
- `GET /events?from&to&source`（新しい絞り込み）→ `events`（lib.rs:3508）の最初の文が `authorize` → `parse_period` → 2 つの SELECT は**どちらも** `core.event_live`、値はすべて bind（`$1`〜`$3`）。ルートは既存の `/events`（lib.rs:3650）で、新しい入口は無い（4）
- 認証の前に落ちるのは axum の `Query` の取り出しだけ: `?from=a&from=SECRETDUP` を資格情報なしで送ると 400 `Failed to deserialize query string: duplicate field \`from\``（値は返さない・記録は読まない）。期間の誤りは認証の後で 400 `invalid_period`（値は返さない）（4）
- `/ingest` の新しい 3 欄 → `validate_end`（ingest.rs:192）→ `truncate_end_to_micros` → INSERT / UPDATE / 履歴の INSERT の `$19`〜`$21`・`$15`〜`$17`・`$12`〜`$14` に bind。断るときの応答は `{"error":"invalid_end_time"}` だけで、送った TZ 名・原文を返さない（4）
- 終わりだけ違う再送 → `is_update`（lib.rs:1579）→ `apply_external_update` の先頭の `stored.deleted_at.is_some()` で止まる。外部の識別子の無い側は `event_dedup_hash` で畳まれて列を動かさない（3。既存の試験 `st41_dedup_deleted_*` 2 本に加え、下の台本でも確認）
- 本文を消去したが論理削除していない行への「終わりだけ違う」更新は本文を書き戻しうるが、そういう行を作る経路がアプリに無い（`raw = ''` を書く非試験のコードは 0 件。`grep -rn "raw = ''" crates/server/src` はコメントだけ）（2）
- 移行で作り直した 3 つのビュー: `event_live` は `deleted_at IS NULL` のまま、`event_version_live` は親の `deleted_at IS NULL` と親の `sensitivity` のまま、`stay_erased` は `raw` / `payload` を出さないまま（202610231200_event_end_time.sql:39-68 と元の版 202609081618 / 202609120943:85 / 202609271716:56 の突き合わせ）。GRANT・ビューのオプションを持つ版は無く、置き換えで落ちるものは無い（2）
- 書き出し（`export.rs` の列・`excluded.rs`）は終わりの列を足しただけで、読む元は `event_live` / `event_version_live` のまま（2）
- 書庫の読み直し（archive/worker.rs:1091）は終わりを `None` で入れる —— 書庫の中身から終わりを写す経路は無い（2）
- ログ: 新しい `tracing` の呼び出しは無い。DB の誤りは既存の `internal_at` を通る（2）
- 待ち受け・接続先・依存: 差分に listen・外への送信・`Cargo.toml` / `Cargo.lock` / npm の変更は無い（`git diff --stat` で 0 件）（2）

安全の決め手: **期間の読み出しの 2 つの枝はどちらも `core.event_live`（`deleted_at IS NULL`）を引き、`authorize` の後でしか走らない** —— 確かさ 4。
`end_time_tests.rs` の末尾に一時の試験 3 本を足して `scripts/quiet-run sec -- cargo test -p ashiato-server sec_st41 -- --nocapture` → 3 passed（試験は走らせた後に消した）:
資格情報なしの期間・ソース・壊れた期間の求めは 401 `unauthorized`、終わりが期間に重なる記録は削除の前は `contains=true`・`soft_delete` の後は `contains=false body=[]`、
ずれ 99999・TZ `SECRET/TZ`・原文 `SECRETRAW` の到着は 400 `[{"…","accepted":false,"error":"invalid_end_time"}]`（`SECRET` を含まない）。

指摘なし

範囲外: 重複したクエリの鍵（`?from=a&from=b`）は認証の前に axum の `Query` が 400 を返す（記録は読まず値も返さないので漏れではない。`/events` 以外の `Query` を取る口も同じ形）。

## /code-review（6bb4584..9935e83）

見た: Claude Code の組み込みの `/code-review`（7 件）。写しは scripts/code_review_record.py

## R17. This section runs `DROP SCHEMA core CASCADE` and reapplies every version, but it does not reapply `crates/server/src/grants.sql`. Every grant to `ashiato_app` that the body set up just before the `.d/` loop (check-immutable.sh:1183-1186) is gone for any section that runs after it.
- 成果物: tools/check-immutable.d/202610231200_st41-event-end-time.sh:5
- 根拠: A later Story adds a `.d/` section that sorts after 202610231200 and fires through `app_psql`, which the body comment says is allowed. Every statement then fails with permission denied. A check like "the app role cannot UPDATE X" passes for the wrong reason (permission denied, not the lock), and a positive check fails with a false NG. The README now tells future sections to copy this exact pattern.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D15 —— ST41 の節は schema を作り直した直後に `./tools/db-roles.sh` と `grants.sql`（`ashiato_owner`）を当て直す。`tools/check-immutable.d/README.md` にも「作り直すと付与も消える。当て直す」を書いた

## R18. `event_end_sql` (used by `overlaps_erased_sql` and late-arrival marking) still reads the end only from `payload->>'end_time'` and ignores the new `end_time` column. `export/excluded.rs` and `core.stay_erased` now read the column first, so the same record gets a different end depending on which path reads it.
- 成果物: crates/server/src/stay_store.rs:699
- 根拠: `/ingest` now accepts a collected interval record whose end is only in the new column, with event_time 00:50 and end_time 02:00. A deleted stay covers 01:00-01:30. Export drops the record because `overlap_sql` reads the column, but the deletion and late-arrival path treats it as a point at 00:50 and leaves it visible in `/events`. Export and reads disagree about the deleted time.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 —— `stay_store::event_end_sql` も終わりの列（始まりより後のとき）を先に読む（書き出しと同じ順）。`/ingest` は書庫の区間のソース（`c03-timeline-visit` など）も受けるので、列だけに終わりを持つ行は起こりうる。試験 `deletion_tests::st41_mark_late_reads_end_column_of_visit`。点のソース（基準のソース）を始まりで見るのは design D22（ST12）のままで、ST41 では変えない

## R19. `core.stay_erased.end_at` uses `coalesce(end_time, payload end, ...)` without the `end_time > event_time` guard that `overlap_sql` has, and `excluded.rs:211` does the same for erased stay versions. A column end before the start makes the span empty, and the payload end is never read. This is R16 in review/code.md and is still unfixed.
- 成果物: migrations/202610231200_event_end_time.sql:59
- 根拠: Take a deleted `s01-stay` with event_time 01:00, end_time 00:30 and payload `{"end":"02:00Z"}`. `stay_erased` gives end_at 00:30 < start_at, so records inside 01:00-02:00 are not excluded from export.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 —— R16 と同じ直し（同じ指摘の再掲）

## R20. In the frozen-column loop, all three end columns are rewritten to `col="end_time"` with the same value. The `end_tz_offset_min` / `end_tz_id` iterations therefore fire the same UPDATE as `end_time`, which is always rejected by the end_time check. The lock lines that freeze the end offset and TZ in `reject_claim_rewrite` are never exercised by this script.
- 成果物: tools/check-immutable.sh:899
- 根拠: Delete the `IF NEW.end_tz_offset_min IS DISTINCT FROM ... OR NEW.end_tz_id ...` block from `reject_claim_rewrite`. `check-immutable.sh` still reports "凍結すると宣言した列はどれも書き換えられない", so the offset/TZ freeze it claims to verify can disappear without notice.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 —— `tools/check-immutable.sh` の凍結の点呼は、終わりの 3 列を「終わりを持つ主張を同じまとまりで置き、その 1 列だけを変えて ROLLBACK」で 1 列ずつ撃つ（終わりを持つ主張を置けること自体も確かめる）。主張の錠からずれと地域の行を外した木で rc=1「NG 主張の end_tz_offset_min / end_tz_id が書き換えられた」を確かめて戻した

## R21. The migration copies the full bodies of five trigger functions (`require_version_or_ledger`, `reject_version_rewrite`, the claim, feeling and place locks), about 300 lines, only to add the end columns. Because it sits last in ORDER and `migrate()` reapplies every version on each startup, these copies override whatever body an earlier version in ORDER defines.
- 成果物: migrations/202610231200_event_end_time.sql:145
- 根拠: A parallel Story changes `reject_feeling_rewrite` (or the gate) in a new migration whose ORDER line lands before 202610231200 after a rebase or union merge. On every startup ST41's stale copy runs last and silently reverts that change, with no conflict and no failing check. Only tests aimed at that specific lock would notice.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D15 —— 移行の 4 の頭で、置き換える 5 関数の当てる直前の本体の md5 が「写した元（と `.down.sql` が戻した本体）」か「この版の本体」のどちらでもなければ `RAISE EXCEPTION` で止める。使い捨ての DB で、全版の当て直し・2 回当て・戻して当て直しは通り、`reject_feeling_rewrite` を別の本体にした後の当ては止まることを確かめた

## R22. The `($1::text IS NULL OR logical_source = $1)` filter is a catch-all predicate. Once sqlx's cached prepared statement switches to a generic plan, PostgreSQL cannot use the `(logical_source, event_time)` index for `?source=` queries.
- 成果物: crates/server/src/lib.rs:585
- 根拠: Repeated `GET /events?source=c01-location` calls on a large archive. After about five executions the generic plan wins and every request scans all of `core.event_live`. Two separate SQL strings, with and without the source filter, would keep the index usable.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D10 —— `GET /events` は `?source=` が有るときだけ `WHERE logical_source = $n` を足す SQL にした。再現: 20 万行で `PREPARE … ($1 IS NULL OR logical_source = $1)` を NULL で 6 回当てた後の `EXPLAIN EXECUTE q('r22-a')` が Seq Scan（`generic_plans=2`）

## R23. `st41_ingest_end_does_not_change_hash` repeats `st41_hash_with_end_is_pinned` from the same diff: same request, same end fields, same pinned hash literal.
- 成果物: crates/server/src/ingest.rs:491
- 根拠: The pinned hash is copied in two places. When the hash input changes, both literals must be updated in step, and the second test adds no coverage.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 —— `st41_ingest_end_does_not_change_hash` を消し、長さ 0 の終わりの場合を `st41_hash_with_end_is_pinned` の中に移した（固定値は 1 か所）
