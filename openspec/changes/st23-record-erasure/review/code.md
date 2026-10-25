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
- 処置:

## R2. D14「範囲の行が無いことは起動時に件数だけログに出す」がどの Task にも無く、作られていない
- 成果物: `crates/server/src/lib.rs`（`run()`）
- 根拠: `openspec/changes/st23-record-erasure/design.md:234`。tasks.md :71 / :100 は読み方の側だけを持つ（Task 3 findings ⚠️）
- kind: technical
- 処置:

## R3. 後着の消去で、台帳の前の版の数と `scope='version'` の行が書かれない
- 成果物: `crates/server/src/deletion.rs`
- 根拠: `crates/server/src/deletion.rs:1046` `purge_late_arrival` は `versions = 0` 固定で `scope='version'` の台帳の行を書かないが、`erase_bodies` は `core.event_version` も消す。書庫の見回り（`mark_archive_arrivals` で本文も消す側）の候補には、以前に `user:late` が付いて前の版を持つ行も入る。門は event_id の台帳の行しか見ないので落ちず、FR-51 の台帳が黙って少なく残る（Task 6 F1）
- kind: technical
- 処置:

## R4. 書庫の位置の組（`Group::ArchiveLocation`）を通る試験が無い
- 成果物: `crates/server/src/deletion.rs`, `crates/server/src/purge_tests.rs`
- 根拠: `purge_scope` の caused のループで組の判定順を入れ替えると、書庫の位置が `locations` に数えられる。確認と消すが同じ関数なので `CountsChanged` も起きず、本人に見せる件数が黙ってずれる（Task 2 F1）
- kind: technical
- 処置:

## R5. 古いスナップショットが残っていると、書き直しが「済み」と記録されても古い本文がディスクに残る
- 成果物: `crates/server/src/purge_compaction.rs`
- 根拠: `crates/server/src/purge_compaction.rs:211`。`VACUUM FULL` は古いスナップショットから見える死んだ行（RECENTLY_DEAD）を新しい表に写す。消去より古いスナップショット（長いトランザクション・idle in transaction・prepared xact・レプリケーションスロット）があると、`relfilenode` は替わって `rewritten` と記録され、`has_unrewritten_erasure` が次の晩の対象から外す。画面の「ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます」が黙って偽になる。D11 が塞いだ「黙った成功」と同じ形
- kind: technical
- 処置:

## R6. ST30 への申し送りの案 (b) が同じ項の前提と矛盾する
- 成果物: `docs/handoff/ST30.md`
- 根拠: `docs/handoff/ST30.md:19` の (b)「戻した後に台帳から消去を再適用する（erasure_span を読む）」は、同じ項の「消去の台帳も同じ時点に戻る」と矛盾する。戻した D-01 にはバックアップ後の消去の台帳が無い（Task 10 F2 の向きの読み違えと同根）
- kind: technical
- 処置:

## R7. `docs/handoff/ST23.md` の済みの項に「残るのは…10.1 が持つ」が残っている
- 成果物: `docs/handoff/ST23.md`
- 根拠: `docs/handoff/ST23.md:101`。10.1 は済んでいる（Task 10 F3）
- kind: technical
- 処置:

## R8. すでに本文を消した滞在では、確認の `range` が始まりの一点に潰れる
- 成果物: `crates/server/src/deletion.rs`
- 根拠: `crates/server/src/deletion.rs:577` が `span_of(event_time, '{}')` になり始まりと終わりが同じになる。D1 は滞在の時間を見せると書く。`core.stay_erased` から読めば直る（Task 4 F2）
- kind: technical
- 処置:

## R9. openapi の `remains` の説明が Rust の定数名を契約に出し、4 つの値がスキーマに出ない
- 成果物: `crates/server/src/lib.rs`
- 根拠: `crates/server/src/lib.rs:2025` の doc コメント「消えずに残るもの（`PURGE_REMAINS`）」が openapi の description になる（Task 8 F2）
- kind: technical
- 処置:

## R10. 複数の滞在がまとまった「消した」行は先頭の滞在だけを本文ごと消す
- 成果物: `web/src/DayView.tsx`
- 根拠: `web/src/DayView.tsx:408` が `stay_ids[0]` だけを渡す。1 回ごとに区間が割れて次が先頭に来るので繰り返せば消せ、確認に出る範囲が先頭の滞在だけであることは文面で分かる
- kind: technical
- 処置:

## R11. 派生の門に WHEN 句が無く、`core.event` のすべての UPDATE が遅延トリガに積まれる
- 成果物: `migrations/202610240500_record_purge.sql`
- 根拠: Task 1 F2。gates.sql の既存のトリガも同じ形
- kind: technical
- 処置:

## R12. 切り詰めの試験は、錠が取れないときトリガが有効かだけを見る
- 成果物: `crates/server/src/purge_tests.rs`
- 根拠: `crates/server/src/purge_tests.rs:34`。fallback で `tgtype & 2`（BEFORE）と関数名も見るか、`testdb::scratch()` に移す（Task 10 F1）
- kind: technical
- 処置:

## R13. 外部識別子の更新で本文を消した時間帯へ動いた行は settle しない
- 成果物: `crates/server/src/lib.rs`（取り込み）
- 根拠: Task 5 F5。`Update::Applied` で時刻が時間帯へ移った行は後着の印も消去も受けない。範囲外とするならコメントに書く
- kind: technical
- 処置:

## R14. 申し送り ch01 D9 の前提「製品のコードに滞在の本文を消す経路は無い」が ST23 の merge で崩れる
- 成果物: `docs/handoff/ST23.md`（main 側で開始後に増えた項）, `crates/server/src/feelings.rs`
- 根拠: `crates/server/src/feelings.rs:294` は payload が `{}` だと `end` が None で `stay_end = stay_start` に縮む。panic・漏れは無いが、受け付けが「区間が始まりの一点を含む」に狭まる。表示は `Mark::UserDeleted` で出ない（ST22 と同じ）。吸収済みの滞在を `user:purged` に付け替えるので、吸収先へたどって出ていた気分が出なくなる（`provisional-review.md:11` 既記）。紐づけ先の規則は ST17 / CH01 の spec の持ち物で、本人は CH01 Q3 で「直さない」を選んでいる。ST23 では直さず PR 本文の未処置の申し送りに書く
- kind: defer
- 処置:
