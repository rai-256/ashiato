## final review（268fa88..5e0d836）

読み方: review package（`.superpowers/sdd/tasks/review-268fa882..5e0d8361.diff`、438KB）を範囲に分けて読んだ。
全 Task の findings の Minor と ⚠️、ledger の parked（Task 3 F1、Ruling 済み）、`docs/handoff/ST23.md` の開始後の増分（main 側の ch01 D9・st09 R8）を triage した。
判定: With fixes（Critical なし）。

### Strengths

- 消去の門が DB 側で閉じている。`require_derived_erasure_ledger`（`migrations/202610240500_record_purge.sql`）は gates.sql の R95 と同じ穴（原文が空のまま payload だけ差し替える）を塞ぎ、台帳と前の版が同じまとまりかを COMMIT の時に見る。本文を消すのは `erase_bodies` の 1 か所だけ。
- 確認と消すが同じ `purge_scope` を使い、件数が違えば `CountsChanged` で何も書かない。すでに本文ごと消した滞在は 200・0 件で返る。
- 時間帯の読み手を `core.stay_erased` に寄せ、D13（`user:purged` を出さない）と D14（範囲の行が無い滞在はその日全体）をビュー 1 つで持たせた。
- 夜の書き直しは `relfilenode` を前後で比べ、権限不足の「警告だけで成功」を失敗として拾う。`.down.sql` は操作の行があれば止まる。

### triage（Task の Minor / ⚠️ / parked / 申し送り）

- Task 4 の `copies: 0` / `remains: []`: Task 8 で本物になった（`deletion::kept_archive_copies` と `PURGE_REMAINS`）。済み。
- Task 3 の後着の本文を消す処理と D14 の窓: `purge_late_body` は `cause.operation_id.filter(|_| cause.purged)` が None なら印だけで戻る。済み。
- Task 1 F5（既存の移行 `202609271716_deletion_ledger.sql` の書き換え）: 許容。`check-immutable.sh` は DB の行を見る検査で移行ファイルを見ない。サーバは起動のたびに全版を当てるので、古い `CREATE OR REPLACE VIEW` のままでは ST23 後の DB で列が減る作り直しになって落ちる。前例 `483df500` / `9faee621`。PR 本文に理由を書く。
- Task 6 の `archive/worker.rs`: ST23 は触っていない（`git diff --stat 268fa882..HEAD -- crates/server/src/archive/` が空）。
- Task 9 F3（`counts.locations`）: `purge_scope` の手順 2 が `e.raw <> ''` で絞るので、本文の残っている件数だけを数える。e2e の代用は成り立つ。
- Task 9 F1（二度押し）: 2 回目は already_purged で 200・0 件。一覧の読み直しが 2 回起きるだけ。Minor のまま。
- Task 3 F1（parked）: Ruling で閉じている（きれいな DB で `stay_` 114/114・`export_excluded` 24・`purge_range_` 9）。
- ほかの Minor（Task 2 F2–F4、Task 4 F1/F3/F5–F7、Task 5 F4、Task 6 F2–F5、Task 7 F4–F7、Task 8 F3/F4、Task 9 F2/F4–F7、Task 10 F4）: merge 前に直す必要は無い。Task 2 F1 は R4、Task 6 F1 は R3、Task 4 F2 は R8、Task 8 F2 は R9、Task 10 F1 は R12、Task 5 F5 は R13、Task 10 F3 は R7 に上げた。

## R1. main との衝突があり、合わさった除外の SQL は誰も試験していない
- 成果物: `crates/server/src/export/excluded.rs`, `crates/server/src/lib.rs`
- 根拠: `git merge-tree origin/main HEAD` で 2 ファイルが衝突。lib.rs は `run()` の組み立て（main の `.with_media(media)` と ST23 の `purge_compaction::spawn`）で機械的。excluded.rs は doc コメントの衝突だが、本体は CH02 の `overlap_sql`（`span_next` の窓）の上に ST23 の `UNION ALL`（`erasure_span` の版）が乗る形になる。Task 10 の「全体の試験に 76 分」は CH02 が同じ遅さを直しているので古い懸念の可能性が高い
- kind: technical
- 処置: rejected: この段では統合しない。main との衝突の解決と、合わさった `overlap_sql` の試験の走らせ直し（`export_excluded_` / `purge_range_` / `purge_keep_`）と全体の試験の時間の測り直しは、確認バッチの conflict_fix が統合の木で行う。PR 本文の未処置に書く

## R2. D14「範囲の行が無いことは起動時に件数だけログに出す」がどの Task にも無く、作られていない
- 成果物: `crates/server/src/lib.rs`（`run()`）
- 根拠: `openspec/changes/st23-record-erasure/design.md:234`。tasks.md :71 / :100 は読み方の側だけを持つ（Task 3 findings ⚠️）
- kind: technical
- 処置: fixed 3.1 - `deletion::purged_without_span` を `run()` で 1 回数え、0 でなければ件数だけ `warn!(kind="purge_span_missing")`。試験 `purge_range_startup_counts_purged_stays_without_a_span_row`（4367c21e）

## R3. 後着の消去で、台帳の前の版の数と `scope='version'` の行が書かれない
- 成果物: `crates/server/src/deletion.rs`
- 根拠: `crates/server/src/deletion.rs:1046` `purge_late_arrival` は `versions = 0` 固定で `scope='version'` の台帳の行を書かないが、`erase_bodies` は `core.event_version` も消す。書庫の見回り（`mark_archive_arrivals` で本文も消す側）の候補には、以前に `user:late` が付いて前の版を持つ行も入る。門は event_id の台帳の行しか見ないので落ちず、FR-51 の台帳が黙って少なく残る（Task 6 F1）
- kind: technical
- 処置: fixed 6.1 - 消す前に前の版を数えて `versions` に入れ、前の版ごとに `scope='version'` の台帳の行を積む。試験 `purge_late_archive_arrival_with_versions_ledgers_each_version`（b76dadc0）

## R4. 書庫の位置の組（`Group::ArchiveLocation`）を通る試験が無い
- 成果物: `crates/server/src/deletion.rs`, `crates/server/src/purge_tests.rs`
- 根拠: `purge_scope` の caused のループで組の判定順を入れ替えると、書庫の位置が `locations` に数えられる。確認と消すが同じ関数なので `CountsChanged` も起きず、本人に見せる件数が黙ってずれる（Task 2 F1）
- kind: technical
- 処置: fixed 2.1 - 試験 `purge_scope_archive_location_is_counted_in_its_own_group`（60aaacda）

## R5. 古いスナップショットが残っていると、書き直しが「済み」と記録されても古い本文がディスクに残る
- 成果物: `crates/server/src/purge_compaction.rs`
- 根拠: `crates/server/src/purge_compaction.rs:211`。`VACUUM FULL` は古いスナップショットから見える死んだ行（RECENTLY_DEAD）を新しい表に写す。消去より古いスナップショット（長いトランザクション・idle in transaction・prepared xact・レプリケーションスロット）があると、`relfilenode` は替わって `rewritten` と記録され、`has_unrewritten_erasure` が次の晩の対象から外す。画面の「ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます」が黙って偽になる。D11 が塞いだ「黙った成功」と同じ形
- kind: technical
- 処置: fixed D11 - `old_snapshot_remains` が書き直しの前後で、窓内の最新の操作の `txid` より古いスナップショット（`pg_stat_activity` / `pg_prepared_xacts` / `pg_replication_slots`）を探し、あれば `not_rewritten` で次の晩にやり直す。試験 `purge_compaction_old_snapshot_is_not_rewritten_and_retried`（f868c3fa）

## R6. ST30 への申し送りの案 (b) が同じ項の前提と矛盾する
- 成果物: `docs/handoff/ST30.md`
- 根拠: `docs/handoff/ST30.md:19` の (b)「戻した後に台帳から消去を再適用する（erasure_span を読む）」は、同じ項の「消去の台帳も同じ時点に戻る」と矛盾する。戻した D-01 にはバックアップ後の消去の台帳が無い（Task 10 F2 の向きの読み違えと同根）
- kind: technical
- 処置: fixed 10.1 - 向き（バックアップより後に消した記録の本文が戻る）と、(b) は台帳を D-01 のバックアップと別に拠点外へ持つ必要があることを書いた（8688e0bb）

## R7. `docs/handoff/ST23.md` の済みの項に「残るのは…10.1 が持つ」が残っている
- 成果物: `docs/handoff/ST23.md`
- 根拠: `docs/handoff/ST23.md:101`。10.1 は済んでいる（Task 10 F3）
- kind: technical
- 処置: fixed 10.1 - 担保しているのは 10.1 でなく `archive_flow_pending_copies_are_removed_after_rereading_when_copies_are_off`（置き場のファイルを数える）と書き直した（8688e0bb）

## R8. すでに本文を消した滞在では、確認の `range` が始まりの一点に潰れる
- 成果物: `crates/server/src/deletion.rs`
- 根拠: `crates/server/src/deletion.rs:577` が `span_of(event_time, '{}')` になり始まりと終わりが同じになる。D1 は滞在の時間を見せると書く。`core.stay_erased` から読めば直る（Task 4 F2）
- kind: technical
- 処置: fixed 4.1 - 本文を消した滞在は時間を `core.stay_erased` から読む。試験 `purge_scope_preview_of_a_purged_stay_keeps_the_range`（220aee9f）

## R9. openapi の `remains` の説明が Rust の定数名を契約に出し、4 つの値がスキーマに出ない
- 成果物: `crates/server/src/lib.rs`
- 根拠: `crates/server/src/lib.rs:2025` の doc コメント「消えずに残るもの（`PURGE_REMAINS`）」が openapi の description になる（Task 8 F2）
- kind: technical
- 処置: fixed 8.1 - `PurgeRemain` の enum にし、4 つの値と利用者向けの説明を契約に出した。`docs/openapi.json` を再生成（94fcc8eb）

## R10. 複数の滞在がまとまった「消した」行は先頭の滞在だけを本文ごと消す
- 成果物: `web/src/DayView.tsx`
- 根拠: `web/src/DayView.tsx:408` が `stay_ids[0]` だけを渡す。1 回ごとに区間が割れて次が先頭に来るので繰り返せば消せ、確認に出る範囲が先頭の滞在だけであることは文面で分かる
- kind: technical
- 処置: rejected: `web/src/DayView.tsx:408` は先頭の滞在を渡し、`web/src/DayView.tsx:413` の戻す口は `stay_ids` 全部を渡す。繰り返せば全部消せ（1 回ごとに区間が割れて次が先頭に来る）、確認に出る範囲が先頭の滞在だけであることは文面で分かる。誤って消す側には倒れない

## R11. 派生の門に WHEN 句が無く、`core.event` のすべての UPDATE が遅延トリガに積まれる
- 成果物: `migrations/202610240500_record_purge.sql`
- 根拠: Task 1 F2。gates.sql の既存のトリガも同じ形
- kind: technical
- 処置: rejected: `migrations/202609120944_gates.sql:150-153`（`event_requires_version`）と `migrations/202610240500_record_purge.sql:155-159` は同じ WHEN 句なしの遅延トリガの形で、正しさの問題でない。WHEN 句で絞ると門の抜けを作りうるので、性能が問題になったときに測ってから直す

## R12. 切り詰めの試験は、錠が取れないときトリガが有効かだけを見る
- 成果物: `crates/server/src/purge_tests.rs`
- 根拠: `crates/server/src/purge_tests.rs:34`。fallback で `tgtype & 2`（BEFORE）と関数名も見るか、`testdb::scratch()` に移す（Task 10 F1）
- kind: technical
- 処置: fixed 10.1 - `(tgtype & 2)`（BEFORE）と `tgfoid = core.reject_<table>_change` を錠の取れ方に依らず常に見る（60aaacda）

## R13. 外部識別子の更新で本文を消した時間帯へ動いた行は settle しない
- 成果物: `crates/server/src/lib.rs`（取り込み）
- 根拠: Task 5 F5。`Update::Applied` で時刻が時間帯へ移った行は後着の印も消去も受けない。範囲外とするならコメントに書く
- kind: technical
- 処置: fixed 5.1 - 範囲外とし、理由（前の版は本人が確認で見た範囲に無い・spec の後着は新しい記録だけ・作り直しで `user:late` が付き書庫なら見回りが消す）を `settle_arrival` の呼び出しにコメントした（667eedb2）

## R14. 申し送り ch01 D9 の前提「製品のコードに滞在の本文を消す経路は無い」が ST23 の merge で崩れる
- 成果物: `docs/handoff/ST23.md`（main 側で開始後に増えた項）, `crates/server/src/feelings.rs`
- 根拠: `crates/server/src/feelings.rs:294` は payload が `{}` だと `end` が None で `stay_end = stay_start` に縮む。panic・漏れは無いが、受け付けが「区間が始まりの一点を含む」に狭まる。表示は `Mark::UserDeleted` で出ない（ST22 と同じ）。吸収済みの滞在を `user:purged` に付け替えるので、吸収先へたどって出ていた気分が出なくなる（`provisional-review.md:11` 既記）。紐づけ先の規則は ST17 / CH01 の spec の持ち物で、本人は CH01 Q3 で「直さない」を選んでいる。ST23 では直さず PR 本文の未処置の申し送りに書く
- kind: defer
- 処置: followup ST17 - PR 本文の「未処置の申し送り」に、受け付けが始まりの一点に縮むことと吸収先へたどる表示が消えることを添えて書く（CH01 Q3 の「直さない」のまま）

## scoped re-review（8cfa442..8688e0b）

R2〜R9・R12・R13 すべて ADDRESSED、fix の diff に新しい Critical / Important なし。Minor: `old_snapshot_remains` が失敗として返るのは SQL エラーのときだけ（偽陽性は次の晩に回るだけでデータを失わない）。

## final の再呼び出し（8688e0b..d539f1d）

final review（268fa88..5e0d836）・1 回の fix・scoped re-review は上で済んでいる。その後の commit は 86a4761（処置の記録と `docs/handoff/ST17.md` への R14 の申し送り）と d539f1d（R10・R11 の根拠の行）の文書だけで、コードは変わっていないので final reviewer を起こし直さない（fix は 1 回だけ）。
申し送りの読み直し: `git diff HEAD origin/main -- docs/handoff/ST23.md` の main 側の増分は ch01 D9 と st09 R8 の 2 件で、新しい項は無い。ch01 D9 は R14。st09 R8（本文の消去を D-02 の原本とバックアップへ及ぼす）は項の宛先のとおり新しい Story が持ち、ST23 の下流では作らない。PR 本文の「未処置の申し送り」に R14 と並べて書く。

## code-verify（HEAD 9c6f2459。独立検証）

席: code-verify（申告と実態のずれ）。最後にコードが変わった commit は 667eedb2、その後の 4 commit（8688e0bb〜9c6f2459）は文書だけ。**全項目の検証コマンドを HEAD で走らせ直した**（10.1 の `cargo test -p ashiato-server` だけは走り切っていない。表の 10.1 と R20）。
変異と探りは作業ツリーの外の複製（`/tmp/st23cv/mut`。HEAD の作業ツリーを rsync した独立の git、`target/` も別）で当て、作業ツリーのコードは触っていない。
`tools/check-immutable.sh`（終了時に `docker compose down -v`）は、この木の DB を消さないよう複製の別 compose project・`ASHIATO_DB_PORT=55791` でだけ走らせた。

実測の前の注記（環境）: 最初の `cargo test -p ashiato-server purge` は 95 本が `testdb.rs:84` の `pool timed out` で落ちた。`target/debug/deps/ashiato_server-7a48ae5e85f1f8bd` が
final review の基準の木 `/tmp/st23-base`（`git worktree list` に残る detached の worktree。8cfa4420）の `CARGO_MANIFEST_DIR` で作られていて、`testdb::db_port()` が `/tmp/st23-base` で `tools/ports.sh` を呼び
55608（別の台帳の番号）へ繋いでいた（`PATH` に差し込んだ偽の `bash` で `cwd=/tmp/st23-base … out=55608` を確かめた）。`cargo clean -p ashiato-server` で作り直すと 55523 を引く。
evidence.jsonl の 10.1 の `cargo test` の PASS（2026-10-25T00:23、HEAD 74067954）はこの実行体ができた 12:44 より前なので影響を受けていない。後の段で同じ木の `cargo test` を作り直しなしに走らせると同じ落ち方をする（`/tmp/st23-base` を外すか `cargo clean -p` が要る）。
1 回目の全体の試験は、試験の DB に過去の実行の行が溜まった状態（`core.event` 16 万行）で書き出しの 16 本が 1 本 12 分以上かかり 1 時間で 598 本に留まったので止め、`tools/db.sh down -v` → `up` → `tools/db-roles.sh` で作り直してから全部を走らせ直した。
1 回目で落ちた `day_tests::st25_reasons_before_collection_started` と `export_excluded_tests::export_excluded_sql_version_of_an_excluded_parent_is_excluded` は、作り直した DB で単独に走らせると両方 1 passed。

**10-27 の走らせ直し（同じ席の続き）:** 10-25 の作業は 10.1 の `cargo test -p ashiato-server` の結果を書く前に止まっていた（表に結果を書く欄が空のまま残っていた。`/tmp/st23cv` は消えていた）。HEAD は 9c6f2459 のままでコードは変わっていないので、変異と探り（R15〜R18）はやり直さず、次だけ走らせ直した。
`plan_fix.py check` → rc=0（`[OK] 機械が承認した検証コマンドの訂正 1 件は…`）/ `plan_fix.py pending` → `[]` / `check_scenarios.py . st23-record-erasure` → rc=0（`scenarios: OK`）/ `cargo test -p ashiato-server purge_keep_` → 7 passed（実行体は `/home/yosis/dev/ashiato2-st23/crates/server` の `CARGO_MANIFEST_DIR` で作られていて、port は 55523）/ origin/main 1bb66dc8（10-27 14:43）で R1・R19 を測り直した（R1: `git merge-tree --write-tree origin/main HEAD` は rc=1 で、衝突は前と同じ `crates/server/src/export/excluded.rs` と `crates/server/src/lib.rs` の 2 つ）。
DB の作り直し（`tools/db.sh down -v`）は auto mode の分類器に止められたので、全体の試験は溜まった DB（始めたとき `core.event` 54,300 行、走らせている間に 77,469 行）の上で走らせた。試験 DB が溜まる問題は ST23 の範囲外で、宛先は CH05（`feat/ch05-test-db-bloat`「試験の DB が打ち切った run の残りで膨れないようにする」）。

### 申告: 12/12（`- [x]` 12・`- [ ]` 0）・検証コマンド一覧

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD 9c6f2459） | 一致 |
|---|---|---|---|
| 1.1 | `check-migrations.sh` / `check-immutable.sh` / `tools/check-st23-derived-erasure.sh`（訂正後の入口）/ `CT purge_schema_ (≥10)` / `CT export_ (≥1)` / `CT app_role (≥1)` | rc=0 / rc=0（複製）/ rc=0・`OK 節の OK 行 36 本`（複製。ただし R17）/ 15 本・通過 / 131 本 / 3 本（全体の試験に含む） | 一致 |
| 2.1 | `CT purge_scope_ (≥17)` / `CT deletion_ (≥1)` | 22 本・通過 / 31 本（全体） | 一致 |
| 3.1 | `CT purge_range_ (≥9)` / `CT export_excluded (≥1)` / `CT stay_ (≥1)` | 9 本・通過 / 24 本・123 本（全体。溜まった DB では 1 本落ち、単独では通る） | 一致 |
| 4.1 | `CT purge_api_ (≥22)` / `CT api_ (≥1)` / openapi の生成 → `check-openapi.sh` / `jq -e …` | 22 本・通過 / 127 本（全体）/ 生成物は `docs/openapi.json` と同一・rc=0（複製）/ `true`・rc=0 | 一致 |
| 5.1 | `CT purge_late_ (≥9)` / `CT deletion_` / `CT dedup_` | 9 本・通過 / 31 本・36 本（全体） | 一致 |
| 6.1 | `CT purge_archive_ (≥3)` / `CT archive_ (≥1)` / `git diff --quiet origin/main -- crates/server/src/archive/worker.rs` | 3 本・通過 / 207 本（全体）/ **rc=1**（R19） | **不一致** |
| 7.1 | `CT purge_compaction_ (≥4)` | 6 本（うち `compaction_volume_run_once` は ignore）・通過 | 一致 |
| 7.2 | `scripts/quiet-run volume -- tools/st23-compaction-volume.sh` | rc=0・`seconds_total=8.8`（core.event 7.9 秒・349,831,168 → 316,416,000 B / core.event_version 0.8 秒）・`limit_seconds=300` | 一致 |
| 8.1 | `CT purge_keep_ (≥7)` | 7 本・通過 | 一致 |
| 9.1 | `CT purge_day_ (≥4)` / `CT day_ (≥1)` | 5 本・通過 / 156 本（全体） | 一致 |
| 9.2 | `E2E st23-purge.spec.ts (≥13)` を `STACK_RESET` なしで 2 回 / `E2E day-erase.spec.ts (≥1)` / `cd web && npm test` | 印 13・13 passed rc=0 を 2 回続けて / 印 3・3 passed rc=0 / 48 files 462 passed rc=0 | 一致 |
| 10.1 | `smoke.sh` / `check-shared-appends.sh` / `check_scenarios.py` / `cargo test -p ashiato-server` / clippy `-D warnings` / `grep -c 'C9' docs/handoff/ST30.md` | rc=0（`OK 本文ごと消す（ST23）`）/ rc=0 / rc=0（`scenarios: OK`）/ **走り切らず**: 10-27 15:50 に始め、背景の上限（2 時間）で 17:50 に止まった。1,305 本のうち 611 passed・1 failed（`day_tests::st25_reasons_before_collection_started`。R20）・残り約 690 本は未実行（書き出しの試験 `export_file_*` が 1 本 8〜10 分で 16 本の枠を埋めていた）。ログ `.superpowers/sdd/logs/20261027-155052-full.log`。残りを埋めるため `cargo test -p ashiato-server -- --skip export_` を走らせた（18:16〜18:26、619 秒）: **1171 passed・2 failed・1 ignored・131 filtered out**（ログ `*-noexport.log`）。落ちたのは R20 の 1 本と `stay_tests::stay_rebuild_chain_of_short_moves_settles`（`stay_tests.rs:2377` の「`rebuild_day` が 3 秒未満」の assert。`2026-05-01 の作り直しが隣の滞在をたどっている`）。後者は単独で 2 回走らせて 2 回とも 1 passed（試験全体で 2.1〜2.3 秒）—— 16 本並びの負荷で 3 秒を超えた。ST23 は `stay_store.rs` を 430 行変えているが、遅くなったのが ST23 の差分かは切り分けていない（10-25 の evidence の全体の試験では通っている）。`export_*` の 131 本は HEAD で走り切っていない（溜まった DB で 1 本 8〜10 分）。evidence.jsonl の PASS（1299 passed・4,550 秒）は HEAD 74067954 で、その後の fix の 6 commit（〜667eedb2）より前 / rc=0 / 2 | **不一致**（HEAD での全体の rc=0 を確かめられず、落ちる 1 本がある。R20） |

`purge` を含む名前の試験は全部で 102 passed・1 ignored（563 秒。溜まった DB の上）。`CT` の本数は `cargo test -p ashiato-server -- --list` の `: test$` を前置きで数えた。

**手 1（固定値の再計算）: 該当なし（一致）。** 確かめた範囲: `purge_compaction_next_run_is_0330_tokyo` の 4 つの境界（00:00 / 03:29:59 / 03:30 / 23:59 → 次の 03:30）を python の `zoneinfo`（Asia/Tokyo）で計算し直し、4 つとも一致。
D14 の日の窓（`date_trunc('day', t AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Asia/Tokyo'`）は 2026-10-01T01:00+09:00 で 2026-09-30T15:00Z〜2026-10-01T15:00Z になり、試験の「15:00（Asia/Tokyo）= 06:00Z はその日の中」と合う。
`purge_schema_*_row_shape_is_fixed` の列・型・NULL 可否・CHECK は、実装より前に書かれた design D4 / D11 の表（列名・`integer NOT NULL CHECK >= 0`・`xid8`・`outcome` の 4 値）と一致する（実装の出力を写しただけの値ではない）。

**手 2（ガードを壊す）:** 複製で 1 か所ずつ壊して `tools/check-st23-derived-erasure.sh` を走らせた。
- 派生の門の「原文だけ空にする書き換えは拒む」分岐を外す → rc=1（`NG 解析済みを残して原文だけ空にする書き換えが通った`）
- `erasure_span_no_truncate` のトリガを作らない → rc=1（`NG 消去の操作の台帳を消せた・切り詰められた: TRUNCATE core.erasure_span;`）
- **ST23 の節のファイルの中身を空にする → rc=0（`OK 節の OK 行 25 本`）（R17）**

**手 3（Scenario と test）:** `check_scenarios.py` は rc=0（`Scenario 1576 件 / 担保あり 1576 / 人間の確認待ち 0`。ST23 の足した 91 本を含む）。warn の `本文を消した滞在へ本文は書き戻せない（原文を空のまま解析済みだけ差し替える形）`（`purge_tests.rs:435`）は spec の名前に括弧書きを足した印で、同じ Scenario は別の試験と `check-immutable.d` の節が正しい名前で持つ。
主張の階層を 1 件ずつ見た範囲 —— 本文の空を `raw` と `payload` の両方で読む（`emptied`）、「本文を持ったまま確定しない」は台帳の `txid` と取り込みのトランザクションの `xmin` を比べる、「確認を求めても何も消えない」は範囲の全記録の `raw` を前後で比べる、
写しは置き場のファイルの SHA-256 を前後で比べる、画面は件数を文面の数字で読み 44 px を `boundingBox` で測る —— は THEN を観測している。ずれは「夜の書き直し」の Scenario と試験（R16）に出た。

**手 4（本人の決定）:** 複製で値・分岐を 1 つずつ変えて関係する試験を走らせた。
- 落ちる（固定されている）: Q1 消えていない滞在を断る分岐を外す（`purge_api_` 3 本）/ Q2 重なる作り直しの滞在を範囲から外す（`purge_scope_absorbed_stay_is_emptied_with_its_versions`）/
  Q3 後着の本文を消さない（`purge_late_` 5 本）/ Q4 確認の写しの数を 0 にする（`purge_keep_` 2 本）/ Q6 03:30 → 04:30（`purge_compaction_next_run_is_0330_tokyo`）。Q5 の 44 px・並び・文面は e2e が `boundingBox` と文面で見ている（上の 9.2）
- **固定されていない・逆が固定されている: Q6 の「その日に本文を消した表だけ」（R16）。Q7・Q8 はどこにも実装が無い（R15）**

**手 5（`[x]` と実体）:** 12 件とも本文の検証コマンドが実在し、`CT` の絞り込みは全部 `-- --list` で下限以上。HEAD で rc≠0 になるのは 6.1 の `git diff --quiet origin/main -- …/worker.rs`（R19）と、10.1 の `cargo test -p ashiato-server`（溜まった試験 DB で 1 本落ちる。走り切ってもいない。R20）。

**手 6（隙間）:** 夜の書き直しが、錠を待っていた消去を「書き直し済み」の窓に入れて取りこぼす経路（R18）。本人が決めた消す道が無いまま本文が残り続ける経路（R15）。
`docs/handoff/ST30.md:14` の C9（バックアップと書庫の写し）は ST30（`docs/stories/ST30.md`「毎日と毎週のバックアップが回る」）に宛てて実在する。

**手 7（機械が承認した訂正）:** 1 件（1.1、2026-10-23T23:42:23+00:00）。門の分岐を壊すと新しい入口は rc=1 —— 「落ちた」。`correction-review.md` に書いた。入口が節の空を捕まえないことは R17。

## R15. 第 2 回の本人の決定 Q7・Q8 が spec・design・tasks・コードのどこにも入っておらず、決めた振る舞いが起きない
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md / specs/browsing-views/spec.md / design.md（D2・D12・D17）/ tasks.md / crates/server/src/stay_store.rs（`rebuild_day`・`day_view`）/ web/src/DayView.tsx
- 根拠: 複製に探りの試験を足して実行した。
  Q7 —— 消した滞在 P（10:00–11:00）にすっぽり入る別に消した滞在 B（10:15–10:45）を置き、P を本文ごと消してから `day_view` を読むと、「消した」系の行は `purged=true`・`stay_ids=None` の 1 行だけで、**B に届く行が無い**（`b_reachable_from_day=false`）。B の本文は残る（`b_kept=true`）。
  Q8 —— P を本文ごと消した後、一覧に出ていた重なる滞在 L で `rebuild_day` を通すと、L は `rebuild:absorbed` で隠れ**本文が残る**（`L_body_kept=true`）。`kind='late'` の操作は 0 件、L の確認は `Err(NotErased)`、P の確認をやり直しても全部 0 件 —— どの口からも L の本文を消せない。
  deep.md の Q7・Q8 は「写す先: … 次の `spec`」とあるが、spec に該当の Requirement・Scenario が無い（`grep 'すっぽり\|隠れたら'` 0 件）。design.md は D2「未決（Q7。仮は否）」・D17「Q8 の答え待ち」のまま。tasks.md に Q7・Q8 の語が無い。
  上流の整合の確認 `docs/audit/2026-10-24-st23-q8-consistency.md:17`（C1・重大度 高）は「印のみ」で残し、f90d1cb2 は「spec 段を通らなかったグラフの問題として harness に送る」と書いた。`go.json:247` は合意の時点で「未解決の 2 件」と本人に見せている。FR-50 の ★ 2026-10-23（`docs/requirements.md:699-702`）は Q8 を要件に戻した
- kind: premise
- 提案: merge の前に、Q7・Q8 を spec（Requirement と Scenario）・design（D2・D12・D17 の書き直し）・tasks に写す段を走らせ、実装と試験を足す（Q8 は `rebuild_day` が隠した重なる滞在を `kind='late'` で同じまとまりで消す、Q7 は P の行の下に B の行を出す）。
  merge を先にするなら、未実装のまま出すことを人間が決める（Q7・Q8 は本人の決定なので AI が範囲外にしない）
- 処置: escalated - deep.md 第 3 回 Q9（Q7・Q8 が作られていないまま merge するか。`deep-questions-r3.json`・`docs/briefs/ST23-deep-r3.html`）。finish は凍結した spec・tasks を書けず、本人の決定を範囲外にもしない

## R16. 夜の書き直しは、本文を消していない表も書き直し、試験はその逆を固定している（Q6「その日に本文を消した表だけ」）
- 成果物: crates/server/src/purge_compaction.rs:143-161（`has_unrewritten_erasure`）/ crates/server/src/purge_compaction_tests.rs:54-59・98-123
- 根拠: `has_unrewritten_erasure(pool, table, now)` は `table` を `core.purge_compaction` の最後の成功を引くのにだけ使い、`core.erasure_operation` の行があれば表を問わず真を返す（前の版を消したか・`versions` が 0 かを見ない）。
  `purge_compaction_rewrites_tables_after_erasure` の `erase_body` は `scene_basic`（前の版を 1 件も積まない滞在）を本文ごと消すだけなのに、`TABLES` の両方について「ファイルが替わる」「`rewritten`」を assert し、この試験は HEAD で通っている（`purge` の 102 passed に含む）。
  探り（R18 の試験）の 1 晩目も、前の版を持たない E1 だけで `core.event_version` を `rewritten` にした。spec は「その前の書き直しの後に本文を消した表だけを書き直す」、deep.md Q6 の答えは「毎晩、その日に本文を消した表だけを書き直す」。
  後着の消去（`kind='late'`。位置 1 件・`versions=0`）が 1 件あるだけでも、毎晩 `core.event_version` に `ACCESS EXCLUSIVE` を取って書き直す。記録は失われない
- kind: technical
- 提案: 表ごとに「その表の本文を消した操作」で絞る（`core.event_version` は `versions > 0` の操作だけ）。試験は前の版の無い消去で `core.event_version` が書き直されない（ファイルが替わらない・行を足さない）ことを足して、いまの assert を直す
- 処置: fixed D11 - `pending_ops_sql`（`crates/server/src/purge_compaction.rs`）が `core.event_version` を前の版を消した操作（`versions > 0`）に絞る。試験 `purge_compaction_skips_version_table_when_no_version_was_erased` を足し、ほかの試験の消去は前の版を 1 件持つ滞在に替えた。窓の SQL を元の形に戻すとこの試験が落ちることを確かめた（7f4338aa）

## R17. `tools/check-st23-derived-erasure.sh`（1.1 の訂正後の入口）は、ST23 の節の中身を空にしても rc=0 になる
- 成果物: tools/check-st23-derived-erasure.sh:9-12
- 根拠: 複製で `tools/check-immutable.d/202610240500_st23-derived-erasure.sh` を `# emptied` の 1 行にして走らせると `OK 節の OK 行 25 本`・**rc=0**。
  OK 行の数え方（`grep -c "  OK .*\(台帳\|書き戻\|作り直し\|原文だけ\)"`）が `check-immutable.sh` の全節をまたぐ —— 壊さないときの 36 本のうち ST23 の節は 11 本（見出しから次の見出しまでを `awk` で切って数えた）で、他の節だけで 25 本あり「10 本以上」を満たす。
  節の見出しの grep は、`check-immutable.sh` がファイルごとに見出しを出すので中身が空でも通る。門そのものを壊せば NG の行で落ちる（`correction-review.md`。手 7 の結果は「落ちた」）
- kind: technical
- 提案: OK 行を節の見出しから次の見出しまでに絞って数える（`awk '/書き換え禁止の節: .*_st23-derived-erasure/{f=1;next} /書き換え禁止の節/{f=0} f'`）。下限は節の OK 行の数（いま 11）にする
- 処置: fixed 1.1 - `tools/check-st23-derived-erasure.sh` は節の見出しから次の見出しまでの OK 行だけを数え、下限を 11 本にした。`scripts/verify-run 1.1` → `OK 節の OK 行 12 本`・PASS（7f4338aa）

## R18. 錠を待っていた消去は、待つ間に走った夜の書き直しの「済み」の窓に入り、その表は以後書き直されない
- 成果物: migrations/202610240500_record_purge.sql:20（`at timestamptz NOT NULL DEFAULT now()`）/ crates/server/src/purge_compaction.rs:119-121・143-161
- 根拠: 操作の時刻 `at` はトランザクションの始まり（`now()`）、書き直しの「最後の成功」は書き直しを始めた時の `clock_timestamp()`、次の晩の対象は `op.at > max(started_at)` の操作。
  複製に探りの試験を足して実行した（使い捨ての DB）: E1 を本文ごと消す → 同じ利用者の錠を別の接続で握る → 前の版を 1 件持つ滞在 E2 の本文ごと消す操作を始める（錠で待つ）→ `run_once`（1 晩目）→ 錠を放す → E2 が commit → `run_once`（2 晩目）。
  結果: 1 晩目 `[core.event: lock_timeout, core.event_version: rewritten]`、E2 は前の版の本文を 1 件消した（`emptied_versions=1`）が `at`（08:55:51）は `core.event_version` の書き直しの始まり（08:55:56）より前、2 晩目は `[core.event: rewritten]` だけ。
  **E2 が消した前の版の古い本文は、別の消去が来て `core.event_version` が書き直されるまでファイルに残る**（来なければずっと）。`old_snapshot_remains` は E2 を見ない（窓の中の操作の `txid` より古いスナップショットを探すが、E2 の操作は窓の外）。
  確認の文面「ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます」が黙って偽になる（R5 と同じ形）。錠を握るのは作り直し・戻す・取り込みの後着の印付けで、03:30 にそれらと消去が重なったときに起きる
- kind: technical
- 提案: 窓の比べ方を「書き直しの始まりより後に commit した操作」にする（例: 書き直しの前に `pg_current_snapshot()` を取り、`txid` がそのスナップショットから見えない操作を次の晩の対象に残す）。探りの手順をそのまま試験にする
- 処置: fixed D11 - 窓を時刻でなく、最後の成功が `VACUUM FULL` の直前に取った `pg_current_snapshot()`（`core.purge_compaction.snapshot` を足した）で切る。それから見えない操作が次の晩の対象に残る。探りの手順をそのまま試験 `purge_compaction_erasure_committed_after_rewrite_is_rewritten_next_night` にした（錠で待たせた消去の前の版を 2 晩目に書き直す）。時刻の窓に戻すと落ちることを確かめた（7f4338aa）

## R19. 6.1 の `git diff --quiet origin/main -- crates/server/src/archive/worker.rs` は HEAD で rc=1。D9 が頼る取り込み器の読み直しは ST13 で形が変わった
- 成果物: openspec/changes/st23-record-erasure/tasks.md（6.1 の検証）/ crates/server/src/archive/worker.rs（main 側）
- 根拠: `git diff --quiet origin/main -- crates/server/src/archive/worker.rs` → **rc=1**（origin/main 1bb66dc8。`git diff --shortstat HEAD origin/main -- …/worker.rs` で main 側が 282 行足し・108 行消し）。差は main 側で、`git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/server/src/archive/` は空 —— ST23 は触っていない（D9 は守っている）。
  ただし main の worker.rs（ST13 の edb66634 ほか）は `mark_archive_arrivals` が落ちたとき `count_reread_failure` / `record_store_failure` を呼び、「3 回続いたら `store_failed` を 1 行書いて 1 時間に 1 回へ落とす」（`git show origin/main:crates/server/src/archive/worker.rs` の 1818-1822・2641-2654 行。1bb66dc8 で測った）。
  `purge_archive_failed_purge_is_done_on_the_next_scan` は ST13 前の worker.rs の上でしか通っていない。design D9 の「見回り（既定 120 秒）ごとに消し直す」は、合わさった木では 3 回目以降 1 時間ごとになる
- kind: defer
- 提案: 6.1 の検証を merge-base との比較（`git diff --quiet $(git merge-base origin/main HEAD) HEAD -- …`）に直す（`plan_fix.py` の手順で）。確認バッチの conflict_fix（R1）が統合の木で `purge_archive_` を走らせ直し、design D9 の再試行の間隔を書き直す
- 処置: fixed D9 - 分岐点と比べる入口 `tools/check-st23-archive-untouched.sh` を足し（rc=0、`OK archive/worker.rs は分岐点（268fa882）から変わっていない`）、D9 に ST13 と合わさった木での間隔（3 回目以降 1 時間ごと）を書いた。6.1 は `python3 scripts/evidence.py unverified openspec/changes/st23-record-erasure --task 6` → `["6.1"]` の項目（本文から検証コマンドを機械で抜き出せない）で、`verify-run 6.1 --command …` が「本文の検証コマンドではない（[]）」で断るので plan_fix の手順に乗らず、tasks.md は直していない。統合の木での `purge_archive_` の走らせ直しは確認バッチの conflict_fix（PR 本文の未処置）

## R20. 全体の試験の `day_tests::st25_reasons_before_collection_started` は、`c01-location` の収集開始日が 2001-01-05 以前になった試験 DB では必ず落ちる（10.1 の `cargo test -p ashiato-server` が rc=0 にならない）
- 成果物: crates/server/src/day_tests.rs:1443-1470 / crates/server/src/coverage.rs:320-326（`touch_started_on`）/ crates/server/src/lib.rs:1450（取り込みが呼ぶ）/ crates/server/src/testdb.rs（この木の版は共有の `ashiato` database）
- 根拠: 10-27 の全体の試験で FAILED（10-25 の 1 回目の全体の試験でも同じ 1 本が落ちた）。同じ DB で単独に `scripts/quiet-run st25 -- cargo test -p ashiato-server st25_reasons_before_collection_started` → rc≠0・`0 passed; 1 failed`・
  `day_tests.rs:1469` の `left: []` / `right: [(BeforeStart, None)]`。試験 DB の `core.source` は `c01-location|2000-01-03`、`core.event` に `c01-location` の 2000-01-03 10:30Z の記録が 1 件（`ingest_time` 2026-10-25 09:41。10-25 の実行が残したもの）。
  試験は「`collection_started_on` が NULL のときだけ 2002-01-01 を置く」ので、すでに 2001-01-05 より前の日が入っていると「開始前」が出ない。取り込みは収集開始日を前へ動かすだけ（`coverage.rs:324-325`）なので、一度入った日は試験 DB の中で戻らない。
  10-25 の作り直した DB では単独で 1 passed、evidence の 1299 passed も通っていた —— 落ちるかは試験 DB に何が溜まっているかで決まる。2000-01-03 を書いた試験は特定していない（`unique_day()`（2000〜2004 年）に `c01-location` を置く試験は書き出しの試験と ST23 の `purge_keep_tests` があるが、書き出しの試験の `put` は `core.event` への直接の INSERT で収集開始日を動かさず、ST23 の `purge_keep_tests.rs:209-221` は使い捨ての DB）。
  ST23 の差分は `day.rs` に `purged` などの列を足しただけで、この試験の読みを変えていない
- kind: defer
- 提案: 宛先は CH04（origin/main に merge 済み 7e4d48c1。`origin/main:crates/server/src/testdb.rs:11` の「回の database」—— 試験は実行ごとに新しい database に書き、`ashiato` に書かない）。合わさった木では実行をまたいだ溜まりが無くなるので、
  確認バッチの conflict_fix（R1）が統合の木で全体の試験を走らせ、この 1 本が通ることを確かめる。通らなければ（1 回の実行の中で先に走った試験が 2001-01-05 以前を書く）、ST25 の試験を自分の `logical_source` で立てる change を起こす

## security review（268fa88..9c6f245）

見た経路:
- `GET /stays/purge/preview`・`POST /stays/purge` → `authorize`（`crates/server/src/lib.rs` の `stays_purge_preview_get` / `stays_purge` の先頭。DB に触る前）→ `deletion::purge_preview` / `purge`。`run()` も同じ `router(state)` を使い、試験用の口 `purge_with_span_scope` と `testdb` は `#[cfg(test)]`（4）
- 画面の求め（`web/src/DayView.tsx` の `fetch("/api/stays/purge")`）→ 印 `SameSite=Strict`（`web_session.rs:189`）＋ `Json` の抽出器（`application/json` が要るので、別サイトのフォームからは送れない）→ 同じ `authorize`（3）
- 外から届く値 `stay_id` / `user_id`（uuid に型付け）・`expected`（`deny_unknown_fields`）→ SQL はどれも bind。`format!` で組む SQL（`purge_compaction.rs` の `SET lock_timeout` / `VACUUM FULL {table}`、`stay_store.rs` の `{USER_LATE}`・`{erased}` など）に入るのは定数と表名の固定値だけ（2）
- 利用者違い → `UserMismatch` → 404（存在するかを見分けられない）・ログは `kind` だけ（`lib.rs` の `purge_response`）（2）
- 新しいログ（`purge_unknown_source`・`stays.purge`・`purge_span_missing`・`purge_compaction`・書庫の写しの有無）→ 出るのは論理ソース名・件数・SQLSTATE・表名・`io::ErrorKind` だけ。識別子・時刻・本文は出ない（2）
- 消した本文が戻る経路: 外部識別子の更新 → `apply_external_update` は `deleted_at` があれば `SkippedDeleted`（`lib.rs` の Q11）。本文ごと消す対象は必ず削除の印つき（`purge_scope` の `deleted_at IS NOT NULL` / 消した滞在だけ）、戻す口は `RestoreError::Purged` で 409。同じ内容の再送は内容の鍵で畳まれる。滞在への書き戻しは `require_derived_erasure_ledger` が拒む（3）
- 後から届いた記録: HTTP の取り込み → `settle_arrival`（同じまとまり）／書庫 → `mark_archive_arrivals` が `read` の台帳の行より先に印と本文の消去をし、落ちれば台帳を書かずに読み直す（`archive/worker.rs:1693`・`:2466`）（2）
- 書き出し → `erasure_operation`・`erasure_span`・`purge_compaction` は `Class::Exclude`（`export.rs`）。応答 `PurgePreview` / `PurgeOutcome` は件数・時間の範囲・`remains` の名前だけで記録の値を持たない（2）
- 待ち受け・接続先・依存: 新しい listen も外への送信先も無い。`Cargo.toml` / `Cargo.lock` / `package*.json` / gradle は差分に無い。権限で新しく足したのは `GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app` だけ（VACUUM FULL 用。読める範囲は増えない）（2）

安全の決め手: 本文ごと消す 2 つの口は、どちらもハンドラの先頭で `authorize`（API の token か画面のログインの印）を通し、通らなければ DB に触る前に 401 を返す —— 確かさ 4。
`scripts/quiet-run auth -- cargo test -p ashiato-server requires_credentials` → rc=0、`purge_api_tests::purge_api_preview_requires_credentials ... ok`・`purge_api_tests::purge_api_requires_credentials ... ok`（3 passed。本物の `crate::router` に資格情報なしで求めを送り、401 になることと本文が残ること（`kept`）を見ている。`authorize` を外すと preview は 200 になって落ちる）。ログ `.superpowers/sdd/logs/20261027-194428-auth.log`。

指摘なし

範囲外: 本文を消した後も塩なしの内容の鍵（`content_hash`）が残り、元の原文の候補を持つ人なら照らし合わせられる —— 既知の C1（`design.md:282`）で、ST23 の差分が足したものではない。

## /code-review（268fa88..9c6f245）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R21. run_once fixes the target cutoff at `now` before it waits for exports. The success watermark is `started_at` (clock_timestamp, taken after the wait), so any erasure operation with now < op.at <= started_at is never picked up by any later run.
- 成果物: crates/server/src/purge_compaction.rs:101
- 根拠: At 03:30 an export is running, so run_once polls for 2 hours. During that time the user purges a stay (op.at = 04:10). The 05:30 VACUUM FULL is recorded as 'rewritten' with started_at = 05:30. The next night, has_unrewritten_erasure needs op.at > 05:30, so the 04:10 operation never qualifies. Unless another purge happens later, the dead tuples holding the purged body stay on disk indefinitely, which contradicts the promise that traces disappear at the next 03:30.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 - R18 と同じ直し。`run_once` から締め切りの `now` を外し、窓はスナップショットだけで切る（書き出しを待つ間に commit した消去は、書き直しの直前のスナップショットから見えれば追い出され、見えなければ次の晩に残る）（7f4338aa）

## R22. The watermark compares `erasure_operation.at` (now() of the purge transaction, i.e. when it started) with `purge_compaction.started_at`. A purge transaction that started before the rewrite but committed after it ends up behind the watermark and is skipped permanently.
- 成果物: crates/server/src/purge_compaction.rs:139
- 根拠: Purge tx T1 begins at 03:29:59 and inserts its erasure_operation row (at = 03:29:59). Compaction starts at 03:30:00 (started_at) and VACUUM FULL core.event_version runs before T1 updates event_version. T1 then erases bodies and commits. That table's run is recorded as rewritten with started_at > T1.at, so later nights never rewrite it for T1 and the old event_version tuples survive.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 - R18 と同じ直し。書き直しの前に始まり後に commit した操作は、書き直しの直前のスナップショットから見えないので次の晩の対象に残る。試験 `purge_compaction_erasure_committed_after_rewrite_is_rewritten_next_night` がこの筋書き（7f4338aa）

## R23. On an erased row, 'purge body' targets only `entry.stay_ids?.[0]`, but an erased row can merge several user-erased stays (stay_ids is the sorted, deduped union of causes).
- 成果物: web/src/DayView.tsx:408
- 根拠: The user erased two overlapping or adjacent stays A and B, and they show as one '消した' row with stay_ids [B, A] (UUID order). Clicking 本文ごと消す previews and purges only B, picked by UUID order rather than by anything the user chose. The confirm shows B's range, and A's body and its locations remain while the row looks like it was handled.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: R10 と同じ指摘。`web/src/DayView.tsx:408` は先頭の滞在を渡し、`web/src/DayView.tsx:413` の戻す口は `stay_ids` 全部を渡す。本文ごと消した滞在は別の行になり（design D12。本文を消していない「消した」とはまとめない）、残った滞在は「消した」の行として「本文ごと消す」の口ごと残るので「片付いたように見える」ことはない。確認に出る範囲は消す滞在の時間で、押すたびに次の滞在が先頭に来る

## R24. The archive pass (purge_bodies=true) erases the body of a row that is already `user:late`, choosing the purged stay as the cause. The row's deletion_ledger entry still points at the stay that originally marked it, so restoring that stay is refused with 409 permanently.
- 成果物: crates/server/src/stay_store.rs:969
- 根拠: Location X overlaps erased stay B (not purged) and erased stay A. Before A was purged, rebuild marked X as user:late with ledger cause B. A is then purged, and mark_archive_arrivals erases X's body with cause A and parent A's operation. Later, POST /stays/restore {B} finds X with purged=true and blocked_by_other_cause=false (its only ledger cause is B), so it returns `purged_restore` 409 every time. B can never be restored even though the user never purged B.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.1 - 書庫の見回りが、すでに別の滞在の後着として隠れていた行を本文を消した滞在の後着として消したら、その滞在を原因に台帳の行を足す（`stay_store::ledger_late`）。試験 `purge_late_archive_row_hidden_by_another_stay_keeps_that_stay_restorable` は直す前に `B を戻せない: Err(Purged)` で落ちた（`.superpowers/sdd/logs/20261027-195540-r24-repro.log`）（7f4338aa）

## R25. Swallowed stays relabeled `user:purged` are dropped from both load_existing sources: raw<>'' excludes them, and stay_erased filters out user:purged. Time they covered outside the designated stay's span is therefore no longer occupied or hidden.
- 成果物: crates/server/src/stay_store.rs:343
- 根拠: A rebuild:erased-range stay S spans 10:00–14:00 and overlaps user-erased stay A (11:00–12:00), so S was hidden. After A is purged, S becomes user:purged with an empty body. The next rebuild_day sees neither S nor any erased range over 12:00–14:00, so a new visible stay is derived there from the unpurged locations. Purging therefore reveals time that was hidden before.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D13（`openspec/changes/st23-record-erasure/design.md:228-234`）が選んだ形。本文ごと消した時間帯は指定した滞在の時間だけで、一緒に本文を消した滞在（`user:purged`）の時間で広げない（`migrations/202610240500_record_purge.sql:80`・`:111`）。広げない側の費用として「はみ出しの時間…は生きた記録として入り、一覧に出る（失うものは無い。本人は出てきた滞在を ST22 の口で消し、本文ごと消せる）」と書き、別の席（provisional-review D13）が確かめている。はみ出しで隠れていたのは作り直しの印（`rebuild:erased-range`）で、本人が消した時間ではない

## R26. Every newly stored located HTTP ingest now runs current_criteria and takes the per-user advisory lock inside the ingest transaction, even for users with no erased stays.
- 成果物: crates/server/src/lib.rs:1468
- 根拠: A phone uploads a batch of location points while a long rebuild_day or purge holds the user's advisory lock. Each per-item ingest transaction blocks on pg_advisory_xact_lock until the rebuild commits, so collector uploads stall or time out, and concurrent ingests for the same user are serialized. A cheap EXISTS check on core.stay_erased before taking the lock would avoid this in the common case.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 錠は取り込みと、同じ利用者の消す・本文ごと消す・作り直しを並べるためのもの（design D8（仮）。`crates/server/src/stay_store.rs:1110` で錠を取ってから原因の滞在を読む）。錠の前に `core.stay_erased` の EXISTS で飛ばすと、走っている消去が commit する前の読みで「消した滞在が無い」と判断し、その時間帯に入った行に後着の印も本文の消去も付かない（錠が塞いでいる競合を開け直す）。錠を待つのは同じ利用者の作り直しと消去が走っている間だけで、位置を持たない記録は錠の前に戻る

## R27. day_view runs a separate erasure_operation SUM query for each purged row (N+1) to compute late_purged.
- 成果物: crates/server/src/stay_store.rs:1525
- 根拠: On a day with many purged stays, each /stays/day request makes one round-trip per purged row. A single grouped query (parent_id = ANY(ops) GROUP BY parent_id) or a join would return the same counts in one query.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 - 本文を消した行の後着の数を、操作ごとに 1 回の `GROUP BY parent_id` の問い合わせでまとめて数える（`stay_store::day_view`）。`purge_day_` 5 本が通る（`late_purged` の値を見る試験を含む）（7f4338aa）

## R28. The is_exporting closure for the long-lived compaction task captures a full `App` clone (pool, token, password hash, sessions, reading state), when it only needs the exports handle.
- 成果物: crates/server/src/lib.rs:3802
- 根拠: Everything in App is kept alive for the process lifetime by a background task that only calls exports.running_view(). Capturing just the exports registry, e.g. `let exports = state.exports.clone(); move || exports.running_view().is_some()`, keeps the dependency narrow.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 - 夜の書き直しの仕事に渡すのを書き出しの台帳（`state.exports.clone()`）だけにした（`crates/server/src/lib.rs` の `run()`）（7f4338aa）

## R29. The nightly compaction JoinHandle is dropped, so if the task panics or exits, nightly rewrites stop without any log or restart.
- 成果物: crates/server/src/lib.rs:3801
- 根拠: If the spawned loop panics (for example via an expect in next_run_after or a panic inside sqlx), tokio swallows it because the handle is dropped. VACUUM FULL never runs again until the server restarts, and nothing records why purged bodies stay on disk.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 仕事の panic は黙って消えない —— `crates/server/src/lib.rs:3700` の `std::panic::set_hook` が `kind="panic"` で必ずログに出す。仕事の本体（`crates/server/src/purge_compaction.rs:63-64`）は抜け道の無い `loop` で、1 晩ごとの失敗は `Err` としてログに出して次の晩へ進む。手綱を捨てて起こしっぱなしにするのは書庫の取り込み器（`crates/server/src/lib.rs:3777`）と同じ形で、作り直しの監督はこの Story の範囲で足さない
