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

## code_verify（HEAD d24a16b1。2026-10-23〜24 実測）

席: code_verify（実装者・final reviewer と別）。コードは触っていない。変異は作業ツリーで一時に当て、1 回ごとに `git checkout -- crates/server/src/export/excluded.rs` で戻し、
前後の `git status --porcelain` と `git diff` が両方空であることを確かめた。使い捨ての DB は全部 `DROP DATABASE … WITH (FORCE)` 済み（終わりに `select datname from pg_database where datname like 'ch02_%'` が 0 行）。

### 申告: 18/18 `[x]`・検証コマンド一覧と実測

`CT <名前>` は tasks.md の Global Constraints の定義（`.env` を読んで `scripts/quiet-run ct -- cargo test -p ashiato-server <名前>`、`-- --list` の本数を下限と比べる）。

| 項目 | 検証コマンド（申告） | 実測（rc・本数） | 一致 |
|---|---|---|---|
| 1.3 | `CT ch02_span_legacy`（1 本以上） | `ch02_` 一括で rc=0・19 passed。`-- --list` 1 本 | 一致 |
| 2.2 | `CT ch02_span_`（10 本以上）/ `CT export_excluded`（24 本以上） | 10 本 / 24 本。`ch02_` 一括 rc=0、`export_` 一括 rc=0 | 一致 |
| 3.4 | `CT ch02_same_`（4 本以上） | 4 本。rc=0 | 一致 |
| 4.4 | `CT ch02_scale_`（4 本以上） | 5 本。rc=0（`--nocapture`: 新 N=5004 → 4N=20005（比 4.0）/ 写し N=501002 → 4N=24008003（比 47.9）/ 古い配置 4004 → 16004 / 消した滞在 0 件: 新 0・実行 0 回・写し 6000） | 一致 |
| 5.1 | `CT export_volume_one_run_reports_time_and_memory`（1 本以上） | 1 本。`export_` 一括で rc=0 | 一致 |
| 5.2 | design の表に空欄なし（`grep -c '\| \|'` が 0） | 0。抜き取りで 10 万行・消した滞在ありを使い捨ての DB `ch02_volume_100000_1` で再実行: `seconds=12.5 summary_seconds=2.0`（表は 12.7 / 2.0）。rc=0 | 一致 |
| 6.1 | `CT export_excluded`(24) / `export_file`(43) / `export_job`(38) / `export_ledger`(3) / `export_tests`(18) / `ch02_`(18) | 24 / 44 / 38 / 15 / 18 / 19。全部 `CT export_` と `CT ch02_` の一括の中で rc=0 | 一致 |
| 6.2 | `CT export_`（126 本以上）/ `git status --short` が空 | 148 本・148 passed・rc=0（1933 秒）/ 空 | 一致 |
| 6.3 | `cargo test -p ashiato-server` rc=0 / clippy `-D warnings` rc=0 / `check_scenarios.py . ch02-…` rc=0 | **crate 全体 rc=101（1221 本中 1 本 FAILED: `day_tests::st25_reasons_before_collection_started`）** / clippy rc=0 / check_scenarios rc=0（OK） | **不一致**（R5。原因は CH02 の外） |
| 1.1・1.2・2.1・3.1〜3.3・4.1〜4.3 | 個別の検証は無く、属する Task の末尾の `CT` が見る | 上のとおり | 一致 |

ほか: `openspec validate ch02-export-next-record-window --strict` rc=0。`plan_fix.py pending` は `[]`。

### 手 1: 固定値を独立に再計算する

- `ch02_scale_rows_read_sums_the_event_scans` の期待値 41 / 34: 同じ計画の JSON を Python で同じ規則（`event` / `event_version` の節の `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops`、`CTE erased_spans` の下を除く）で数え直して `41 34`。一致
- D1 の枠そのもの（実装と別に書いた SQL）: 使い捨ての DB `ch02_verify_scratch` に 6000 行（利用者 3・ソース 3・端末 4 と NULL・時刻はマイクロ秒と秒の混在・前の版に当たる `is_event = false` が 2 割）を置き、
  `min(tm) FILTER (WHERE is_event) OVER (PARTITION BY u,s,d ORDER BY tm DESC RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING)` と、相関副問い合わせ
  `min(n.tm) … n.is_event AND n.u=r.u AND n.s=r.s AND n.d IS NOT DISTINCT FROM r.d AND n.tm > r.tm` を突き合わせた → 6000 行中 食い違い 0（同じ時刻の仲間を持つ行 1034・次が無い行 62 を含む）
- `legacy` の写しが逐語か: 元の commit `ab08064` の `excluded_sql` / `overlap_sql` と、`excluded.rs` の `legacy` の同名関数を、行ごとに前後の空白を落として diff → どちらも差 0。
  （base `536d22c` の `excluded.rs` は `ab08064` と同一）。突き合わせの相手は実装の写しでなく、変更前の実物
- `ch02_same_differs_…` の `same_time_first = a.min(b)`（Rust の `Uuid` の順）が PostgreSQL の `uuid` の順と同じ前提: 乱数の UUID で走らせた 8 回（下の変異のうち 3.2 が落ちない回を含む）すべてで緑。順が食い違えば約半分で落ちる

### 手 2: ガードをわざと壊す

`excluded.rs` に 1 つずつ当て、`cargo test -p ashiato-server ch02_` を走らせた（基準: 変異なしで 19 passed・rc=0）。

| 変異（excluded.rs） | rc | 落ちた試験 |
|---|---|---|
| M1 `span_next` の 3 枝の `WHERE EXISTS (SELECT 1 FROM erased_spans)` を外す | 0 | なし（19 passed） |
| M1b `overlap_sql` の `AND EXISTS (SELECT 1 FROM erased_spans)` だけを外す | 0 | なし（19 passed） |
| M1c 上の 4 か所を全部外す | 101 | `ch02_scale_nothing_erased_reads_no_records` |
| M2 組ごとの後の最初の 1 行の枝（`GROUP BY`）を空にする（`AND false`） | 101 | `ch02_same_matches_the_strict_legacy_on_every_family`（`t = '2000-01-01T00:30:00Z'` でだけ） |
| M3 `FILTER (WHERE q.is_event)` を外す（前の版が次の候補になる） | 101 | `ch02_span_version_is_not_the_next_of_another_version`・3.1・3.2 |
| M4 `PARTITION BY` から `device_id` を外す | 101 | `ch02_span_null_device_runs_within_null_devices`・3.1・3.2 |
| M5 枠を `… AND CURRENT ROW` にする（同じ時刻が枠に入る） | 101 | 7 本（同じ時刻の組・4.4・1.3 ほか） |
| M6 `span_next` の記録の 2 枝を `core.event`（印の付いた行も候補）にする | 101 | `ch02_span_marked_record_is_not_the_next`・3.1・3.2 |
| M7 前の版の枝の組の利用者を `v.user_id` にする | 101 | `ch02_span_version_group_uses_the_parent_user`・3.1・3.2 |
| M8 順向きで尾を固定した枠（`ORDER BY q.event_time RANGE BETWEEN '1 microsecond' FOLLOWING AND UNBOUNDED FOLLOWING`） | 101 | `ch02_scale_window_frame_is_head_fixed` だけ（結果の試験は全部緑。design D7 の「読んだ行の数では見分けられない」を実行で確かめた） |

M1 / M1b: Q4 の門は 2 段の重ね（`overlap_sql` の One-Time Filter が偽なら、そこからしか参照されない `span_next` の CTE は評価されない）で、片方だけ外しても読まない。
振る舞い（消した滞在 0 件なら記録の表を読まない）は M1c で落ちるので固定されている。指摘にはしない。
検査の外側: 4.3 の `rows_read(…, Some("erased_spans"))` が `CTE erased_spans` を本当に名前で外しているか —— M1c で非 0 になって落ちたこと、外さなければ `erased_spans` の下の `event` の走査が数に入ることから、名前の一致は効いている。

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch02-export-next-record-window` rc=0（scenarios: OK。warn は全部この change の外のファイル）。
この change の 28 本の印の先: 既存 17 本は `export_file_tests.rs`（16）・`export_job_tests.rs`（1）、新しい 11 本は `export_window_tests.rs`。新しい 11 本を 1 件ずつ WHEN / THEN と配置を見比べた。どれも WHEN の時刻・端末・利用者・座標の有無のとおりに置き、THEN の id を名指しで見ている。
変異で、9 本の幅の試験がそれぞれの Scenario の守る分岐（M3〜M7）で落ちることを確かめた。

階層の違い（指摘にしない）: 新しい 9 本の THEN は「記録のファイル / 前の版のファイルに出ない」、試験は `kept_ids`（`event_kept` / `version_kept` の SQL）で、zip は作らない。
tasks.md 2.1 がこの階層を指定していて、ファイルの階層は既存 17 本（`export_file_tests` がファイルを開いて見る）が新しい断片の上で通ること（`CT export_` 148 passed）と、
書き出しの本体が同じ `ExcludedSql::event_kept` / `version_kept` を使うこと（`dump.rs:138`）で担保される。ADDED の 2 本は「読んだ行の数」「1 度も実行されない」を計画の JSON で直接見ていて、階層は合っている。

### 手 4: 本人の決定が test で固定されているか

| 決定（deep.md） | 書き換えたときに落ちるか |
|---|---|
| Q1 同じ時刻は飛ばし、時刻が後の最初の記録を次にする | 落ちる（M5: 7 本。`ch02_span_same_time_pair_runs_to_the_later_record` ほか） |
| Q2 いまの SQL を試験に残して新旧を突き合わせる | 写しは逐語（手 1）。M2〜M7 のどれも 3.1 か 3.2 が落とす |
| Q3 読んだ行の数を N と 4N で比べる（`SCALE_N = 1_000`・`SCALE_FACTOR = 4`・`SCALE_LIMIT = 8`） | `SCALE_LIMIT` は実測の比（新 4.0 / 写し 47.9）に両側から挟まれ、計算では 5 未満か 48 以上で落ちる（値を書き換えて走らせてはいない）。`SCALE_N` と `SCALE_FACTOR` は試験の物差しそのもので、製品の値ではない |
| Q4 消した滞在 0 件なら窓を回さない | 落ちる（M1c で 4.3） |
| Q4 消した時間の終わりより後は組ごとの最初の 1 行だけを窓に入れる | 落ちる（M2 で 3.1 の `t = 00:30`）。実際の呼び出し（`now()` / `transaction_timestamp()`）では効かない枝で、固定の時刻の族だけが守る（design D3 のとおり） |

### 手 5: tasks の `[x]` と実体

18 項目の検証の入口は全部実在した（上の表）。`cargo test --test <名前>` 型は無い。名前で絞る `CT` は全部 `-- --list` の下限以上（0 本で緑のものは無い）。
6.2 の訂正（`CT st09_exp_` → `CT export_`）は人間の承認つきで、機械の承認ではない（`plan_fix.py pending` が `[]`）。
不一致は 6.3 の crate 全体だけ（R5）。

### 手 6: 隙間

- 消したものが戻らない型: 除く範囲の断片を読むのは `dump.rs:138`（書き出し）・`summary.rs:92`（件数）・`ledger.rs:431`（後に消した件数）の読み出しだけ。`export/` の中で `core.event` に印を付ける・消す SQL は断片を使っていない
  （`grep -n 'UPDATE core.event\|deleted_by' crates/server/src/export/*.rs` は `excluded.rs` の 2 行と `ledger.rs:437` の読み出しだけ）。Q1 で除く側が広がっても、記録を消すことはない
- 写しの時点が戻る（時計が戻る・固定の時刻を渡す）: 3.1 の固定の時刻 3 つ（消した滞在の前・中・後）で新旧が一致。M2 がそこで落ちる
- 時間の上限: server に `statement_timeout` も API のタイムアウトも無い（`grep -rn 'statement_timeout\|TimeoutLayer\|timeout(' crates/server/src` が 0 件）ので、120 万行で 25.5 秒の件数の数えが途中で切れて失われる経路は無い（遅いことは design の D13 の欄と PR 本文にある）
- 6.2 で「確認バッチの統合の木に任せる」とした ST09 の上での `export_`: 任せた先は実在する —— `verify_batch.sh` が統合の HEAD の CI を待ち（`.harness2/scripts/verify_batch.sh:225-261`）、CI は `cargo test --workspace`（`.github/workflows/ci.yml:74`）を走らせる。defer の行き先あり

### 手 7: 機械が承認した検証コマンドの訂正

手 7: 該当なし（確かめた範囲: `python3 scripts/plan_fix.py pending openspec/changes/ch02-export-next-record-window` が `[]`・rc=0。唯一の訂正 6.2 は plan-corrections.md のとおり人間の承認。`correction-review.md` は書いていない）

## R5. 6.3 の `cargo test -p ashiato-server` が、この worktree の試験 DB では rc=101 になる（共有の DB の積もり。CH02 の持ち込みではなく、持ち主が無い）
- 成果物: tasks.md 6.3 / crates/server/src/day_tests.rs:1443 / crates/server/src/testdb.rs:213 / crates/server/src/coverage.rs:322
- 根拠: `scripts/quiet-run ct -- cargo test -p ashiato-server` → rc=101、1221 本中 1220 passed・`day_tests::st25_reasons_before_collection_started` が `left: [] right: [(BeforeStart, None)]`（ログ `.superpowers/sdd/logs/20261023-235851-ct.log:1317`）。
  単独で走らせ直しても FAILED（rc=101）。共有の DB では `core.source` の `c01-location` の `collection_started_on` が `2000-01-15`（`core.event` に 2000-01-15 の `c01-location` が 7 行）で、試験が置く 2001-01-05 は「開始の後」になる。
  `testdb::unique_day()` は 2000〜2004 年の日を配り、`coverage.rs:322` は取り込みのたびに開始日を最も早い日へ下げるので、どこかの試験が 2001-01-05 より前の位置を取り込むと以後この試験は落ち続ける。
  使い捨ての DB `ch02_verify_st25`（全移行だけ）に向けて同じ試験を走らせると ok（rc=0）。CH02 の試験はまっさらな DB で走り、`core.event` に直に入れて `coverage` を通らない。
  evidence.jsonl の 6.3 の crate 全体は 3 回: 2026-10-22T16:06 FAIL（`st25_reasons_before_collection_started` だけ。1220 passed）・2026-10-23T09:06 FAIL（同じ試験 + ST09 の移行が残った回の export の試験）・09:13 PASS（1221 passed・579 秒。今回の 2764 秒より大幅に短く、DB を作り直した直後の回とみられる）。`[x]` は積もりが消えた直後の 1 回に載っていて、同じ 1 本は今回も落ちた。持ち主: `docs/changes/` は CH01（ST17 の /code-review）と CH02 だけ、`docs/stories/` に試験 DB の積もりを持つ Story は無い（design の Non-Goals は「別の change」とだけ書く）
  CI は job ごとに runner の上で `docker compose up -d --wait db` から立てた DB で `cargo test --workspace` を走らせる（`.github/workflows/ci.yml` の server の job。積もりは持ち越さない）。この HEAD の PR はまだ無く（`gh pr list --head feat/ch02-export-next-record-window` が `[]`）、CI の結果は観測していない
- kind: defer
- 提案: CH02 の merge は止めない（CI は毎回新しい DB。merge gate がその HEAD の CI を見る）。`hx change` で「`st25_reasons_before_collection_started` を共有の DB の開始日から切り離す（その試験だけの論理ソースか、まっさらな DB）」を起こす。それまでは、手元の 6.3 の再確認はまっさらな DB で走らせる
- 処置: rejected: CH02 の持ち込みではない —— 同じ試験はまっさらな DB（`ch02_verify_st25`）で rc=0、CI も job ごとに新しい DB（.github/workflows/ci.yml:74）。落ちるのは共有の試験 DB の `core.source` の開始日の積もり（crates/server/src/day_tests.rs:1443 が global な `c01-location` の行に依る）。持ち主の ST25 は archive 済みで handoff の先が無いので、PR 本文の「未処置の申し送り」に `hx change` の起こし方つきで写した

## security review（536d22c..d24a16b）

見た経路:
- 入口: 差分に新しい API・route・コマンド・listen・送信先・依存は無い（`git diff --stat` は `excluded.rs`・試験 3 ファイル・lib.rs の `#[cfg(test)] mod` だけ。`Cargo.toml` / `Cargo.lock` / web / android は 0 行）（2）
- `GET /export/summary` → `summary.rs:92` `excluded_sql("now()")` → `span_next` → `overlap_event` / `overlap_version` → 件数（2）
- 書き出し → `dump.rs:138` `excluded_sql("transaction_timestamp()")` → `event_kept` / `version_kept` → zip に出る行（2）
- 後に消した件数 → `ledger.rs:431` `excluded_sql("now()")`（2）
- SQL に入る値: `snapshot_at` は 3 か所とも固定の文字列、`is_event` は Rust の `bool`、`points` / `END_KEYS` / `DURATION_KEYS` は定数、`REBUILD_PREFIX` は `const`。取り込みの値が SQL の文字列に入る口は増えていない（3）
- `legacy` の写しは `#[cfg(test)] pub(crate) mod legacy`（excluded.rs:373）で本番のビルドに入らない（2）
- 漏れの向き: 消した滞在に重なる記録が、新しい断片で除かれなくなる（= 書き出しのファイル・件数に戻る）経路があるか → 下の決め手で否定（4）
- 試験・evidence の出力: 新しい試験の出力は件数・読んだ行の数だけ。design の 120 万行の表は `export_volume_one_run_reports_time_and_memory` が作る偽の行の秒数で、記録の中身は無い（evidence.jsonl の 5.2 / 6.x の summary を `http|lat|lon|title|url` で見て、試験名と秒数だけ）（3）

安全の決め手: **新しい断片が除く記録・前の版の集合は、旧の断片（`legacy::excluded_sql`）が除く集合を必ず含み、狭めない（違いは同じ時刻の組の先の記録が除かれる側に増えることだけ）** —— 確かさ 4。
一時の試験 `secreview_tmp_random_superset` を `export_window_tests.rs` に足して走らせ、`git checkout` で消した（`git status` は `review/code.md` だけ。一時の DB `st16_tmp_*` の残りは 0）。
中身: まっさらな DB を 1 周につき 1 つ × 6 周、利用者 3・ソース 4（`c02-window` `c01-location` `c02-browser-history` `c01-photo`）・端末 NULL/a/b・記録 400（1/8 に `user` か `rebuild:` の削除印、`end` が壊れた行、`visit_duration_us`、原文の地図 URL の有無）・前の版 80（1/4 は親と違う利用者）・本人が消した滞在 1〜3（前の版つき）と作り直しで消した滞在を乱数で置き、
`snapshot_at` 4 通り（`now()` `transaction_timestamp()` 固定 2 つ）で `old.excluded ⊆ new.excluded`（記録・前の版）・`new.kept ⊆ old.kept`・`new == legacy::excluded_sql_strict` を見る。
秒とマイクロ秒の混ざった配置で seed 1/7/42: `rows_checked=11520`・除外 3216 / 2782 / 3695・`new_only=0`・全部 ok。
同じ時刻が多い配置（5 分刻み・秒 0）で seed 3/99: `new_only=28` / `20`（除く側にだけ増える。Q1 のとおり）・全部 ok。

指摘なし

範囲外: `erased_spans`（excluded.rs:203〜）は利用者で絞らないので、ある利用者の消した滞在が別の利用者の記録も除く（除き過ぎの側で漏れではない。base から変わらない）

## /code-review（536d22c..d24a16b）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R6. The full-table `span_next` window (event and version branches together) is rebuilt in every statement that embeds `excluded_sql`. One export runs it three times (`count_filtered`, the events.jsonl SELECT, the event_versions.jsonl SELECT), plus once each in `read_summary` and the ledger check.
- 成果物: crates/server/src/export/dump.rs:268
- 根拠: A user deletes a recent stay, so max(end_at) is about now() and the window covers about 1.2M rows. Each dump statement sorts all of them by (user, source, device, time), versions included even when only the events side is needed. The sort spills to disk and is paid 3 or more times per export; at 120万行 that is far more than one pass. Building `span_next` once per transaction (a temp table, or one statement) would avoid it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 文ごとに窓を組み直すのは事実だが「1 回よりはるかに重い」は実測で否定 —— design.md:162-163 の 120 万行で消した滞在あり 146.5 秒・無し 97.9 秒、窓の分は書き出し全体で約 49 秒（件数の数え 1 回の 25.5−0.9≈25 秒の約 2 倍）。10 万→120 万で 11.5 倍の線形（design.md:166）。CH02 の完了の判定（2 乗にしない）の外の定数倍で、一時表の案は docs/changes/CH02.md:23（写しは一時表を作らない。ST33 D5）に反する。1 つの transaction で 1 回にまとめる案は ST33 D13 を本人が決めるときの候補として PR 本文に写した

## R7. A failed ledger write is ignored. The `printf ... > tmp && mv` failure is followed by an unconditional `echo "$mine"`, so `_ports_slot` exits 0 with a slot number it never recorded. The header promises '割り当てられなければ（…書けない）非 0 を返す'.
- 成果物: tools/ports.sh:36
- 根拠: The ledger tmp file cannot be written (disk full, which this branch's logs already hit with /tmp, or a permission problem). Worktree A gets slot n without it being recorded. Worktree B then gets the same n, and both use DB 55600+n, recreating the collision the ledger exists to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）。読んだ限り tools/ports.sh:36 の `mv` の失敗は確かに拾われない —— PR 本文の「未処置の申し送り」に写した

## R8. The ledger and its lock live in the git common dir (the main checkout's `.git`), outside the worktree. Every `ports.sh` call for a `-ch`/`-up` worktree opens `$1.lock` for writing, and `testdb.rs` panics when that fails.
- 成果物: tools/ports.sh:39
- 根拠: Codex runs `cargo test` in the `ashiato2-ch02` worktree under a workspace-write sandbox whose writable dirs are the worktree plus ~/.cargo, ~/.gradle and ~/.android. `9>/home/yosis/dev/ashiato2/.git/ashiato-ports.lock` fails, `ports.sh` returns 1, and `db_port()` panics with 'tools/ports.sh が port を決められない'. Every DB test then fails, even when the slot is already in the ledger.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）。PR 本文の「未処置の申し送り」に写した

## R9. `git rev-parse --path-format=absolute` needs git 2.31 or later. On older git the error is swallowed by `|| true`, `_ports_dirs` is empty, and every worktree takes the main-checkout branch (55432 / 18787).
- 成果物: tools/ports.sh:42
- 根拠: On a host with git 2.25, every `-st<NN>`, `-ch<NN>` and `-verify` worktree silently gets DB 55432. They all fight over the main DB, which is exactly the 2026-10-21 failure. The script never fails as its comment says it should ('55432 には落とさない').（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）。この host は `git --version` が 2.53.0 で起きない。PR 本文の「未処置の申し送り」に写した

## R10. `_ports_root` comes from `cd … && pwd`, which gives a logical path. The ledger compares it to stored paths as a plain string, so one worktree reached by a symlinked path and by its physical path gets two different slots.
- 成果物: tools/ports.sh:17
- 根拠: `tools/stack.sh` run from a symlinked shell path records slot 0. `cargo test` spawns bash with cwd `<manifest>/../..`, which resolves to the physical path, so `testdb.rs` gets slot 1. The tests then connect to 55601, where no DB is running. `pwd -P` would avoid the mismatch.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）。tools/ports.sh:17 は `pwd`（論理パス）—— PR 本文の「未処置の申し送り」に写した

## R11. `with_worktree_port` copies `testdb.rs`'s bash/`ports.sh` call but has no OnceLock. Every `env("DATABASE_URL" | "DATABASE_OWNER_URL")` call starts bash and git again and may touch the ledger lock.
- 成果物: crates/server/tests/server_startup.rs:24
- 根拠: Each startup test that reads both URLs starts bash and git 2 or more times, each taking the flock on the shared ledger. That is duplicated logic in two files plus extra process starts and lock contention when tests run in parallel. A shared helper with caching would do the same job once.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）。PR 本文の「未処置の申し送り」に写した

## R12. `overlap_sql` re-checks `EXISTS (SELECT 1 FROM erased_spans)` and `r.event_time <= (SELECT max(end_at) …)`. `span_next` already contains only rows that pass both, and `r` is now joined from `span_next`.
- 成果物: crates/server/src/export/excluded.rs:292
- 根拠: Two extra initplans and filters per branch that can never reject a row. A reader can take them for load-bearing (D3 points at the `span_next` branches, not these). Dropping them makes the fragment simpler with the same result.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 （2 つの条件は結果を変えないのは確か —— `span_next` は `WHERE w.id IS NOT NULL` で後の最初の 1 行を落とし、各枝が同じ門と絞りを持つ（crates/server/src/export/excluded.rs:209-237）。条件は門の重ねとして残し（code_verify の M1 / M1b で片方だけ外しても読まない）、overlap_sql の doc にそう書いた。計画と 5.2 の計測を変えないため SQL は触らない）

## R13. The `versions` subquery still selects `v.event_id`, `e.user_id` and `e.device_id`. After this change `overlap_sql` reads none of them, as its new doc says (rows need only `id` `logical_source` `event_time` `payload` `raw`).
- 成果物: crates/server/src/export/excluded.rs:190
- 根拠: The leftover columns make the subquery look as if the group key is still read per row, which is the old behavior D2 removed. They also widen the join's projection for nothing.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 （`versions` の副問い合わせから `v.event_id` `e.user_id` `e.device_id` を外した。`legacy` の写しは逐語のまま）

## R14. This comment still says recording worktrees fall back to 55432 / 18787 ('録画の worktree の名前は -st<NN> で終わらないので規則は 55432 / 18787'). Under the new `ports.sh`, `-rec-st<NN>` worktrees get a ledger slot (55600+n).
- 成果物: tools/record-env.sh:130
- 根拠: Someone who removes the explicit `ASHIATO_DB_PORT` export, trusting this comment, gets a ledger port instead of the dev DB they expected. A stale comment in shared tooling misleads whoever maintains it next.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: CH02 の差分の外（`git diff --stat 536d22c..HEAD -- tools/ crates/server/tests/` が空。c7a8737f で main に入ったもの。`git merge-base --is-ancestor c7a8737f 536d22c` rc=0）（tools/record-env.sh も同じ）。PR 本文の「未処置の申し送り」に写した

## R15. The branch's commits use a scoped prefix (`docs(CH02): …`, `chore(CH02): …`, `feat(CH02): …`). CLAUDE.md §git states 'コミットは `feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行'.
- 成果物: CLAUDE.md:1
- 根拠: Every CH02 commit (for example d24a16b1 'docs(CH02): review/code.md の処置を…') breaks the stated prefix form. Any tool that matches `^(feat|fix|docs|chore):` will not recognise these commits.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 範囲つきの接頭辞は repo の慣行 —— `git log --format=%s origin/main -15` の 9 本が `docs(st41):` などの形（例 54c121b9）。CLAUDE.md の「`feat:` / … + 日本語 1 行」は種類の規則で、範囲を禁じていない。`.githooks` も止めない
